use std::panic::resume_unwind;
use tokio::sync::Semaphore;

pub struct Parallelism(Semaphore);

impl Parallelism {
    pub fn new() -> Self {
        Parallelism(Semaphore::new(num_cpus::get()))
    }
    pub async fn run_blocking<F: 'static + Send + FnOnce<()>>(&self, x: F) -> F::Output where F::Output: 'static + Send {
        let guard = self.0.acquire().await;
        match tokio::task::spawn_blocking(x).await {
            Ok(x) => x,
            Err(e) => resume_unwind(e.into_panic())
        }
    }
}
