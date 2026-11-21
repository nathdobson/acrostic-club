use crate::llm::ollama_to_anyhow;
use futures::FutureExt;
use futures::future::BoxFuture;
use ollama_rs::Ollama;
use ollama_rs::generation::chat::ChatMessageResponse;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use std::sync::Arc;

pub struct BaseClient {
    inner: Ollama,
}

pub trait ChatClient: Send + Sync + 'static {
    fn send_chat_messages<'a>(
        &'a self,
        input: &'a ChatMessageRequest,
    ) -> BoxFuture<'a, anyhow::Result<ChatMessageResponse>>;
}

impl ChatClient for BaseClient {
    fn send_chat_messages<'a>(
        &'a self,
        input: &'a ChatMessageRequest,
    ) -> BoxFuture<'a, anyhow::Result<ChatMessageResponse>> {
        async move {
            Ok(self
                .inner
                .send_chat_messages(input.clone())
                .await
                .map_err(ollama_to_anyhow)?)
        }
        .boxed()
    }
}

impl BaseClient {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let client = Ollama::default();
        Ok(Arc::new(BaseClient { inner: client }))
    }
}
