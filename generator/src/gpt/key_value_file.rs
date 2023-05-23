use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::{io, mem};
use std::path::Path;
use std::sync::Arc;
use std::thread::panicking;
use parking_lot::Mutex;
use safe_once_async::async_lazy::AsyncLazy;
use safe_once_async::once::AsyncOnce;
use safe_once_async::sync::AsyncOnceLock;
use serde::de::DeserializeOwned;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::Serialize;
use serde::Deserialize;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;
use tempfile::{tempdir, tempfile, TempPath};

pub struct KeyValueFile<K, V> {
    map: Mutex<HashMap<K, Arc<AsyncOnceLock<Arc<V>>>>>,
    sender: Option<UnboundedSender<KeyValueEntry<K, V>>>,
    writer: Option<JoinHandle<io::Result<()>>>,
}

#[derive(Serialize, Deserialize)]
struct KeyValueEntry<K, V> {
    key: K,
    value: V,
}

impl<K: Serialize + DeserializeOwned + Eq + Hash + Clone + Send + 'static,
    V: Serialize + DeserializeOwned + Clone + Send + 'static> KeyValueFile<K, V> {
    pub async fn new(path: &Path) -> io::Result<Self> {
        let mut option = OpenOptions::new();
        option.read(true);
        option.write(true);
        option.create(true);
        let mut file = option.open(path).await?;
        let mut history = String::new();
        file.read_to_string(&mut history).await?;
        let mut map = HashMap::new();
        for line in history.lines() {
            let entry: KeyValueEntry<K, V> = serde_json::from_str(line)?;
            map.insert(entry.key, Arc::new(AsyncOnceLock::from(Arc::new(entry.value))));
        }
        let (tx, mut rx) = unbounded_channel::<KeyValueEntry<K, V>>();
        let writer = tokio::spawn(async move {
            while let Some(x) = rx.recv().await {
                let mut m = serde_json::to_string(&x).unwrap();
                m.push('\n');
                file.write_all(m.as_bytes()).await?;
            }
            Ok(())
        });
        Ok(KeyValueFile { map: Mutex::new(map), sender: Some(tx), writer: Some(writer) })
    }
    pub async fn get_or_init<E>(&self, key: K, value: impl Future<Output=Result<V, E>>) -> Result<Arc<V>, E> {
        let once_lock = self.map.lock().entry(key.clone()).or_default().clone();
        Ok(once_lock.try_get_or_init(async {
            let value = value.await?;
            self.sender.as_ref().unwrap().send(KeyValueEntry { key, value: value.clone() }).ok().unwrap();
            Ok(Arc::new(value))
        }).await?.clone())
    }
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.sender.take();
        self.writer.take().unwrap().await?
    }
}

impl<K, V> Drop for KeyValueFile<K, V> {
    fn drop(&mut self) {
        if self.writer.is_some() {
            eprintln!("Warning: forgot to flush cache");
        }
    }
}

#[tokio::test]
async fn test_key_value_file() -> io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("my-temporary-note.txt");
    let mut kvf = KeyValueFile::<usize, usize>::new(&path).await?;
    assert_eq!(2, *kvf.get_or_init(1, async { io::Result::Ok(2) }).await.unwrap());
    assert_eq!(4, *kvf.get_or_init(3, async { io::Result::Ok(4) }).await.unwrap());
    kvf.shutdown().await?;
    kvf = KeyValueFile::new(&path).await?;
    assert_eq!(2, *kvf.get_or_init(1, async { io::Result::Ok(todo!()) }).await.unwrap());
    assert_eq!(4, *kvf.get_or_init(3, async { io::Result::Ok(todo!()) }).await.unwrap());

    Ok(())
}