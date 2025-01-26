use crate::llm::chat_client::ChatClient;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use futures::future::BoxFuture;
use futures::FutureExt;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use ollama_rs::generation::chat::ChatMessageResponse;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use tokio::time::sleep;
// use crate::llm::types::{ChatRequest, ChatResponse};
use crate::util::rate_limit::RateLimit;

pub struct RateLimitClient {
    inner: Arc<dyn ChatClient>,
    rate: Mutex<RateLimit>,
}

impl RateLimitClient {
    pub fn new(inner: Arc<dyn ChatClient>, rate: RateLimit) -> Arc<Self> {
        Arc::new(RateLimitClient {
            inner,
            rate: Mutex::new(rate),
        })
    }
}

impl ChatClient for RateLimitClient {
    fn send_chat_messages<'a>(
        &'a self,
        input: &'a ChatMessageRequest,
    ) -> BoxFuture<'a, anyhow::Result<ChatMessageResponse>> {
        async move {
            let time = self.rate.lock().spawn();
            tokio::time::sleep_until(time).await;
            let resp = self.inner.send_chat_messages(input).await?;
            Ok(resp)
        }
        .boxed()
    }
}
