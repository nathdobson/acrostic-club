use crate::llm::chat_client::ChatClient;
use crate::llm::key_value_file::KeyValueFile;
use futures::FutureExt;
use futures::future::BoxFuture;
use ollama_rs::generation::chat::ChatMessageResponse;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use std::io;
use std::path::Path;
use std::sync::Arc;
// use crate::llm::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatResponse, ChatRole, Endpoint, Model};
use crate::util::interrupt::CleanupSender;

pub struct CacheClient {
    inner: Arc<dyn ChatClient>,
    cache: Box<KeyValueFile<String, ChatMessageResponse>>,
}

impl ChatClient for CacheClient {
    fn send_chat_messages<'a>(
        &'a self,
        input: &'a ChatMessageRequest,
    ) -> BoxFuture<'a, anyhow::Result<ChatMessageResponse>> {
        self.generate_impl(input).boxed()
    }
}

impl CacheClient {
    pub async fn new(
        x: Arc<dyn ChatClient>,
        path: &Path,
        cleanup: CleanupSender,
    ) -> io::Result<Arc<Self>> {
        let kvf = KeyValueFile::new(path, cleanup).await?;
        Ok(Arc::new(CacheClient {
            inner: x,
            cache: Box::new(kvf),
        }))
    }
    async fn generate_impl(
        &self,
        input: &ChatMessageRequest,
    ) -> anyhow::Result<ChatMessageResponse> {
        let inner = self.inner.clone();
        let input = input.clone();
        Ok((*self
            .cache
            .get_or_init(serde_json::to_string(&input)?, async move {
                inner.send_chat_messages(&input).await
            })
            .await?)
            .clone())
    }
}
