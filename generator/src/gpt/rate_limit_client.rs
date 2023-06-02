use std::sync::Arc;
use std::time::Instant;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use futures::future::BoxFuture;
use futures::FutureExt;
use parking_lot::Mutex;
use tokio::time::sleep;
use crate::gpt::chat_client::ChatClient;
use crate::gpt::types::{ChatRequest, ChatResponse};
use crate::util::rate_limit::RateLimit;

pub struct RateLimitClient {
    inner: Arc<dyn ChatClient>,
    rate: Mutex<RateLimit>,
}

impl RateLimitClient {
    pub fn new(inner: Arc<dyn ChatClient>, rate: RateLimit) -> Arc<Self> {
        Arc::new(RateLimitClient { inner, rate: Mutex::new(rate)})
    }
}

impl ChatClient for RateLimitClient {
    fn chat<'a>(&'a self, input: &'a ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>> {
        async move {
            let time = self.rate.lock().spawn();
            tokio::time::sleep_until(time).await;
            let resp = self.inner.chat(input).await?;
            Ok(resp)
        }.boxed()
    }
}
