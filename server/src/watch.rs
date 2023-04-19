use std::collections::HashMap;
use std::future::poll_fn;
use std::mem;
use std::pin::pin;
use std::sync::{Arc, Weak};
use std::sync::mpsc::RecvError;
use std::task::Poll;
use std::time::Duration;
use parking_lot::Mutex;
use tokio::sync::{broadcast, Notify};
use tokio::time::sleep;

pub struct Watchable<T> {
    state: T,
    receivers: Vec<Weak<Watcher<T>>>,
}

struct Queue<T> {
    state: Option<T>,
    closed: bool,
}

pub struct Watcher<T> {
    queue: Mutex<Queue<T>>,
    notify: Notify,
}

impl<T> Watchable<T> {
    pub fn new() -> Self where T: Default {
        Watchable { state: T::default(), receivers: vec![] }
    }
    pub fn watch(&mut self) -> Arc<Watcher<T>> where T: Clone {
        let result = Arc::new(Watcher {
            queue: Mutex::new(Queue {
                state: Some(self.state.clone()),
                closed: false,
            }),
            notify: Notify::new(),
        });
        self.receivers.push(Arc::downgrade(&result));
        return result;
    }
    pub fn modify(&mut self, mut modify: impl for<'a> FnMut(&'a mut T)) where T: Default {
        modify(&mut self.state);
        self.receivers.retain_mut(|x| {
            if let Some(x) = x.upgrade() {
                modify(x.queue.lock().state.get_or_insert_default());
                x.notify.notify_one();
                true
            } else {
                false
            }
        });
    }
}

impl<T> Watcher<T> {
    pub async fn recv(&self) -> Result<T, RecvError> {
        loop {
            {
                let mut lock = self.queue.lock();
                if let Some(state) = lock.state.take() {
                    return Ok(state);
                }
                if lock.closed {
                    return Err(RecvError);
                }
                mem::drop(lock);
            }
            self.notify.notified().await;
        }
    }
}

impl<T: Default> Default for Watchable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Watchable<T> {
    fn drop(&mut self) {
        for x in &self.receivers {
            if let Some(x) = x.upgrade() {
                x.queue.lock().closed = true;
                x.notify.notify_one();
            }
        }
    }
}


#[tokio::test]
async fn test() {
    let mut sender = Watchable::<usize>::new();
    let r1 = sender.watch();
    let r2 = sender.watch();
    sender.modify(|x| *x += 1);
    let x = tokio::spawn(async move {
        assert_eq!(1, r1.recv().await.unwrap());
        assert_eq!(2, r1.recv().await.unwrap());
        assert_eq!(2, r1.recv().await.unwrap());
        assert_eq!(RecvError, r1.recv().await.unwrap_err());
    });
    let y = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        sender.modify(|x| *x += 2);
        sleep(Duration::from_millis(100)).await;
        sender.modify(|x| *x += 2);
    });
    x.await.unwrap();
    y.await.unwrap();
    assert_eq!(5, r2.recv().await.unwrap());
    assert_eq!(RecvError, r2.recv().await.unwrap_err());
}