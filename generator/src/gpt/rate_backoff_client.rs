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
use crate::util::rate_backoff::RateBackoff;
use crate::util::rate_limit::RateLimit;

pub struct RateBackoffClient {
    inner: Arc<dyn ChatClient>,
    rate: Mutex<RateBackoff>,
}

impl RateBackoffClient {
    pub fn new(inner: Arc<dyn ChatClient>, rate: RateBackoff) -> Arc<Self> {
        Arc::new(RateBackoffClient { inner, rate: Mutex::new(rate) })
    }
}

impl ChatClient for RateBackoffClient {
    fn chat<'a>(&'a self, input: &'a ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>> {
        async move {
            let event = self.rate.lock().spawn();
            tokio::time::sleep_until(event.time).await;
            let resp = self.inner.chat(input).await;
            event.success.get_or_init(|| resp.is_ok());
            resp
        }.boxed()
    }
}
