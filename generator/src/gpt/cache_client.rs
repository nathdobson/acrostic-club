use std::default::default;
use std::{io, mem};
use std::path::Path;
use std::sync::Arc;
use futures::future::BoxFuture;
use futures::FutureExt;
use crate::gpt::chat_client::{BaseClient, ChatClient};
use crate::gpt::key_value_file::{KeyValueFile, KeyValueFileCleanup};
use crate::gpt::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatResponse, ChatRole, Endpoint, Model};
use crate::PACKAGE_PATH;

pub struct CacheClient {
    inner: Arc<dyn ChatClient>,
    cache: Box<KeyValueFile<ChatRequest, ChatResponse>>,
}

impl ChatClient for CacheClient {
    fn chat<'a>(&'a self, input: &'a ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>> {
        self.chat_impl(input).boxed()
    }
}

impl CacheClient {
    pub async fn new(x: Arc<dyn ChatClient>, path: &Path) -> io::Result<(Arc<Self>, KeyValueFileCleanup)> {
        let (kvf, cleanup) = KeyValueFile::new(path).await?;
        Ok((Arc::new(CacheClient { inner: x, cache: Box::new(kvf) }), cleanup))
    }
    async fn chat_impl(&self, input: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let inner = self.inner.clone();
        let input = input.clone();
        Ok((*self.cache.get_or_init(input.clone(), async move {
            inner.chat(&input).await
        }).await?).clone())
    }
}

#[tokio::test]
async fn test_cache_client() -> anyhow::Result<()> {
    let (cache_client, cleanup) =
        CacheClient::new(BaseClient::new().await?,
                         &PACKAGE_PATH.join("build/chat_cache.txt")).await?;
    let response = cache_client.chat(&ChatRequest {
        endpoint: Endpoint::Chat,
        body: ChatRequestBody {
            model: Model::GPT_3_5_TURBO,
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: "You are a dog.".to_string(),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: "hello".to_string(),
                },
            ],
            ..default()
        },
    }).await?;
    mem::drop(cache_client);
    cleanup.cleanup().await?;
    Ok(())
}