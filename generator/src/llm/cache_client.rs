use crate::llm::chat_client::{BaseClient, ChatClient};
use crate::llm::key_value_file::{KeyValueFile, KeyValueFileCleanup};
use futures::future::BoxFuture;
use futures::FutureExt;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::ChatMessageResponse;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use std::path::Path;
use std::sync::Arc;
use std::{io, mem};
// use crate::llm::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatResponse, ChatRole, Endpoint, Model};
use crate::PACKAGE_PATH;

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
    ) -> io::Result<(Arc<Self>, KeyValueFileCleanup)> {
        let (kvf, cleanup) = KeyValueFile::new(path).await?;
        Ok((
            Arc::new(CacheClient {
                inner: x,
                cache: Box::new(kvf),
            }),
            cleanup,
        ))
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