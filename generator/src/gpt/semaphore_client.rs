use std::sync::Arc;
use std::time::Instant;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use futures::future::BoxFuture;
use futures::FutureExt;
use parking_lot::Mutex;
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio::time::sleep;
use crate::gpt::chat_client::ChatClient;
use crate::gpt::types::{ChatRequest, ChatResponse};
use crate::util::rate_limit::RateLimit;

pub struct SemaphoreClient {
    inner: Arc<dyn ChatClient>,
    semaphore: Semaphore,
}

impl SemaphoreClient {
    pub fn new(inner: Arc<dyn ChatClient>, limit: usize) -> Arc<Self> {
        Arc::new(SemaphoreClient { inner, semaphore: Semaphore::new(limit) })
    }
}

impl ChatClient for SemaphoreClient {
    fn chat<'a>(&'a self, input: &'a ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>> {
        async move {
            let guard: SemaphorePermit = self.semaphore.acquire().await.unwrap();
            self.inner.chat(input).await
        }.boxed()
    }
}
