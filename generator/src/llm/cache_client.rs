use crate::llm::chat_client::{BaseClient, ChatClient};
use crate::llm::key_value_file::{KeyValueFile, KeyValueFileCleanup};
use futures::future::BoxFuture;
use futures::FutureExt;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use std::path::Path;
use std::sync::Arc;
use std::{io, mem};
// use crate::llm::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatResponse, ChatRole, Endpoint, Model};
use crate::PACKAGE_PATH;

pub struct CacheClient {
    inner: Arc<dyn ChatClient>,
    cache: Box<KeyValueFile<String, GenerationResponse>>,
}

impl ChatClient for CacheClient {
    fn chat<'a>(
        &'a self,
        input: &'a GenerationRequest,
    ) -> BoxFuture<'a, anyhow::Result<GenerationResponse>> {
        self.chat_impl(input).boxed()
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
    async fn chat_impl(&self, input: &GenerationRequest) -> anyhow::Result<GenerationResponse> {
        let inner = self.inner.clone();
        let input = input.clone();
        Ok((*self
            .cache
            .get_or_init(serde_json::to_string(&input)?, async move {
                inner.chat(&input).await
            })
            .await?)
            .clone())
    }
}

#[tokio::test]
async fn test_cache_client() -> anyhow::Result<()> {
    let (cache_client, cleanup) = CacheClient::new(
        BaseClient::new().await?,
        &PACKAGE_PATH.join("build/chat_cache.txt"),
    )
    .await?;
    let response = cache_client
        .chat(&GenerationRequest::new(
            "llama3.3:70b".to_string(),
            "Are you dog?".to_string(),
        ))
        .await?;
    mem::drop(cache_client);
    cleanup.cleanup().await?;
    Ok(())
}
