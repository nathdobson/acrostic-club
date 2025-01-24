use crate::llm::chat_client::ChatClient;
use backoff::backoff::{Backoff, Zero};
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use futures::future::BoxFuture;
use futures::FutureExt;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
// use crate::llm::types::{ChatRequest, ChatResponse};

pub struct RetryClient {
    inner: Arc<dyn ChatClient>,
    backoff: Box<dyn Sync + Send + Fn() -> Box<dyn Sync + Send + Backoff>>,
}

impl RetryClient {
    pub fn new(
        inner: Arc<dyn ChatClient>,
        b: impl 'static + Sync + Send + Clone + Backoff,
    ) -> Arc<Self> {
        Arc::new(RetryClient {
            inner,
            backoff: Box::new(move || Box::new(b.clone())),
        })
    }
    pub fn new_zero(inner: Arc<dyn ChatClient>) -> Arc<Self> {
        Arc::new(RetryClient {
            inner,
            backoff: Box::new(move || Box::new(Zero {})),
        })
    }
    pub fn new_exponential(inner: Arc<dyn ChatClient>) -> Arc<Self> {
        Self::new(
            inner,
            ExponentialBackoffBuilder::new()
                // .with_initial_interval(Duration::from_secs(1))
                // .with_max_interval(Duration::from_secs_f64(f64::INFINITY))
                // .with_multiplier(2.0)
                .build(),
        )
    }
}

impl ChatClient for RetryClient {
    fn chat<'a>(
        &'a self,
        input: &'a GenerationRequest,
    ) -> BoxFuture<'a, anyhow::Result<GenerationResponse>> {
        async move {
            let mut backoff = (self.backoff)();
            loop {
                match self.inner.chat(input).await {
                    Ok(x) => return Ok(x),
                    Err(e) => {
                        if let Some(backoff) = backoff.next_backoff() {
                            sleep(backoff).await;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
        .boxed()
    }
}
