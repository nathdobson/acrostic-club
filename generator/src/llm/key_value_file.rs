use crate::util::interrupt::CleanupSender;
use crate::util::lazy_async::CloneError;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncOnceLock;
use safe_once_map::sync::AsyncOnceLockMap;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::future::Future;
use std::hash::Hash;
use std::io;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

pub struct KeyValueFile<K, V> {
    map: AsyncOnceLockMap<K, JoinTransparent<anyhow::Result<V>>>,
    sender: UnboundedSender<KeyValueEntry<K, V>>,
}

#[derive(Serialize, Deserialize)]
struct KeyValueEntry<K, V> {
    key: K,
    value: V,
}

impl<
    K: Serialize + DeserializeOwned + Eq + Hash + Clone + Send + Sync + 'static,
    V: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
> KeyValueFile<K, V>
{
    pub async fn new(path: &Path, cleanup: CleanupSender) -> io::Result<Self> {
        let mut option = OpenOptions::new();
        option.read(true);
        option.write(true);
        option.create(true);
        let mut file = option.open(path).await?;
        let mut history = String::new();
        file.read_to_string(&mut history).await?;
        let mut map = AsyncOnceLockMap::new();
        for line in history.lines() {
            let entry: KeyValueEntry<K, V> = serde_json::from_str(line)?;
            let value = entry.value;
            map[&entry.key]
                .get_or_init(spawn_transparent(async move { Ok(value) }))
                .await;
        }
        let (tx, mut rx) = unbounded_channel::<KeyValueEntry<K, V>>();
        cleanup.send(async move {
            while let Some(x) = rx.recv().await {
                let mut m = serde_json::to_string(&x).unwrap();
                m.push('\n');
                file.write_all(m.as_bytes()).await?;
            }
            Ok(())
        });
        Ok(KeyValueFile { map, sender: tx })
    }
    pub async fn get_or_init<'a>(
        &'a self,
        key: K,
        value: impl 'static + Send + Future<Output = anyhow::Result<V>>,
    ) -> anyhow::Result<&'a V> {
        let sender: UnboundedSender<_> = self.sender.clone();
        let cell: &'a AsyncOnceLock<JoinTransparent<anyhow::Result<V>>> =
            self.map.get_or_insert(Cow::Owned(key.clone()));
        let fut = async move {
            let value = value.await;
            if let Ok(value) = &value {
                sender
                    .send(KeyValueEntry {
                        key,
                        value: value.clone(),
                    })
                    .ok()
                    .unwrap();
            }
            value
        };
        let output: &'a anyhow::Result<V> = cell.get_or_init(spawn_transparent(fut)).await;
        Ok(output.clone_error()?)
        // let this = self.clone();
        // let once_lock: Arc<AsyncOnceLock<anyhow::Result<Arc<V>>>> = self.map.lock().entry(key.clone()).or_default().clone();
        // let result = once_lock.get_or_init(|| async move {
        //     let value = value.await?;
        //     this.sender.as_ref().unwrap().send(KeyValueEntry { key, value: value.clone() }).ok().unwrap();
        //     Ok(Arc::new(value))
        // }).await;
        // Ok(result.clone_error()?.clone())
    }
}
