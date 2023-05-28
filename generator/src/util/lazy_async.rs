use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{LazyLock, OnceLock};

use futures::future::BoxFuture;
use futures::FutureExt;
use safe_once_async::sync::{AsyncLazyLock, AsyncStaticLock};
use tokio::io;
use tokio::sync::Mutex;

use crate::util::alloc::{AnyRepr, MmapAllocator, restore_vec};
//
// pub struct LazyAsync<T> {
//     thunk: Mutex<Option<BoxFuture<'static, T>>>,
//     value: OnceLock<T>,
// }
//
// impl<T> LazyAsync<T> {
//     pub fn new<F: 'static + Send + Future<Output=T>>(thunk: F) -> Self {
//         LazyAsync { thunk: Mutex::new(Some(thunk.boxed())), value: OnceLock::new() }
//     }
//     pub async fn get(&self) -> &T {
//         if let Some(value) = self.value.get() {
//             return value;
//         }
//         {
//             let ref mut thunk = *self.thunk.lock().await;
//             if let Some(thunk) = thunk.take() {
//                 self.value.set(thunk.await).ok().expect("unreachable");
//             }
//         }
//         self.value.get().expect("Poisoned")
//     }
// }
//
// impl<T> LazyAsync<io::Result<T>> {
//     pub async fn get_io(&'static self) -> io::Result<&'static T> {
//         match self.get().await {
//             Ok(x) => Ok(x),
//             Err(e) => Err(io::Error::new(e.kind(), e))
//         }
//     }
// }
//
// impl<T> LazyAsync<anyhow::Result<T>> {
//     pub async fn get_anyhow(&'static self) -> anyhow::Result<&'static T> {
//         struct AnyhowRef(&'static anyhow::Error);
//         impl Debug for AnyhowRef {
//             fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//                 Debug::fmt(self.0, f)
//             }
//         }
//         impl Display for AnyhowRef {
//             fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//                 Display::fmt(self.0, f)
//             }
//         }
//         impl std::error::Error for AnyhowRef {
//             fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//                 self.0.source()
//             }
//             #[allow(deprecated)]
//             fn description(&self) -> &str {
//                 self.0.description()
//             }
//             #[allow(deprecated)]
//             fn cause(&self) -> Option<&dyn std::error::Error> {
//                 self.0.cause()
//             }
//             fn provide<'a>(&'a self, demand: &mut std::any::Demand<'a>) {
//                 self.0.provide(demand)
//             }
//         }
//         match self.get().await {
//             Ok(x) => Ok(x),
//             Err(e) => Err(anyhow::Error::new(AnyhowRef(e)))
//         }
//     }
// }

pub struct LazyMmap<T> (AsyncStaticLock<io::Result<Box<[T], MmapAllocator>>>);

pub trait CloneError {
    type Value;
    type Error;
    fn clone_error_static(&'static self) -> Result<&'static Self::Value, Self::Error>;
    fn clone_error(&self) -> Result<&Self::Value, Self::Error>;
}

impl<T> CloneError for io::Result<T> {
    type Value = T;
    type Error = io::Error;
    fn clone_error_static(&'static self) -> Result<&'static Self::Value, Self::Error> {
        self.as_ref().map_err(|e| io::Error::new(e.kind(), e))
    }
    fn clone_error(&self) -> Result<&Self::Value, Self::Error> {
        self.as_ref().map_err(|e| io::Error::new(e.kind(), e.to_string()))
    }
}

struct AnyhowRef(&'static anyhow::Error);

impl Debug for AnyhowRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.0, f)
    }
}

impl Display for AnyhowRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.0, f)
    }
}

impl std::error::Error for AnyhowRef {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.0.description()
    }
    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.0.cause()
    }
    fn provide<'a>(&'a self, demand: &mut std::any::Demand<'a>) {
        self.0.provide(demand)
    }
}


impl<T> CloneError for anyhow::Result<T> {
    type Value = T;
    type Error = anyhow::Error;
    fn clone_error_static(&'static self) -> Result<&'static Self::Value, Self::Error> {
        self.as_ref().map_err(|e| anyhow::Error::new(AnyhowRef(e)))
    }

    fn clone_error(&self) -> Result<&Self::Value, Self::Error> {
        self.as_ref().map_err(|e| anyhow::Error::msg(e.to_string()))
    }
}

impl<T: AnyRepr> LazyMmap<T> {
    pub const fn new(path: fn() -> PathBuf) -> Self {
        LazyMmap(AsyncStaticLock::new(async move {
            restore_vec(&path()).await
        }))
    }
    pub fn get(&'static self) -> impl Send + Future<Output=io::Result<&'static [T]>> {
        async move {
            Ok(&**(self.0.get().await.clone_error_static()?))
        }
    }
}
