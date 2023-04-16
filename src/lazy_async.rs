use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{LazyLock, OnceLock};
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::io;
use tokio::sync::Mutex;
use crate::alloc::{AnyRepr, MmapAllocator, restore_vec};

pub struct LazyAsync<T> {
    thunk: Mutex<Option<BoxFuture<'static, T>>>,
    value: OnceLock<T>,
}

impl<T> LazyAsync<T> {
    pub fn new<F: 'static + Send + Future<Output=T>>(thunk: F) -> Self {
        LazyAsync { thunk: Mutex::new(Some(thunk.boxed())), value: OnceLock::new() }
    }
    pub async fn get(&self) -> &T {
        if let Some(value) = self.value.get() {
            return value;
        }
        {
            let ref mut thunk = *self.thunk.lock().await;
            if let Some(thunk) = thunk.take() {
                self.value.set(thunk.await).ok().expect("unreachable");
            }
        }
        self.value.get().expect("Poisoned")
    }
}

impl<T> LazyAsync<io::Result<T>> {
    pub async fn get_io(&'static self) -> io::Result<&'static T> {
        match self.get().await {
            Ok(x) => Ok(x),
            Err(e) => Err(io::Error::new(e.kind(), e))
        }
    }
}

pub struct LazyMmap<T> (LazyAsync<io::Result<Box<[T], MmapAllocator>>>);

impl<T: AnyRepr> LazyMmap<T> {
    pub fn new(path: PathBuf) -> Self {
        LazyMmap(LazyAsync::new(async move {
            restore_vec(&path).await
        }))
    }
    pub async fn get(&'static self) -> io::Result<&'static [T]> {
        Ok(self.0.get_io().await?)
    }
}
