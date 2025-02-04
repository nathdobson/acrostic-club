use futures::future::BoxFuture;
use safe_once_async::detached::{spawn_transparent, JoinTransparent};
use std::future::Future;
use std::mem;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

type Cleanup = Box<dyn 'static + Send + FnOnce() -> JoinTransparent<anyhow::Result<()>>>;

#[derive(Clone)]
pub struct CleanupSender(mpsc::UnboundedSender<Cleanup>);

pub struct CleanupReceiver(mpsc::UnboundedReceiver<Cleanup>);

pub fn channel() -> (CleanupSender, CleanupReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (CleanupSender(tx), CleanupReceiver(rx))
}

impl CleanupSender {
    pub fn send<Fu: 'static + Sync + Send + Future<Output = anyhow::Result<()>>>(&self, fu: Fu) {
        self.0.send(Box::new(|| spawn_transparent(fu))).ok();
    }
}

impl CleanupReceiver {
    pub async fn cleanup(mut self) -> anyhow::Result<()> {
        while let Some(next) = self.0.recv().await {
            next().await?;
        }
        Ok(())
    }
}

pub async fn run_with_interrupts<
    F: 'static + Send + FnOnce(CleanupSender) -> Fu,
    Fu: 'static + Send + Future<Output = anyhow::Result<()>>,
>(
    f: F,
) -> anyhow::Result<()> {
    let (tx, rx) = channel();
    let () = select!(
        result = ctrl_c() => anyhow::Result::<()>::Ok(result?),
        result = spawn_transparent(f(tx)) => anyhow::Result::<()>::Ok(result?),
    )?;
    let () = select!(
        result = ctrl_c() => anyhow::Result::<()>::Ok(result?),
        result = rx.cleanup() => anyhow::Result::<()>::Ok(result?),
    )?;
    Ok(())
}
