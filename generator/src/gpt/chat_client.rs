use std::any::Any;
use std::default::default;
use std::future::Future;
use std::{io, mem};
use std::path::Path;
use std::sync::Arc;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::Serialize;
use serde::Deserialize;
use tokio::fs;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::time::sleep;
use crate::gpt::key_value_file::{KeyValueFile, KeyValueFileCleanup};
use crate::gpt::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatResponse, ChatResponseResult, ChatRole, Endpoint, Model};
use crate::PACKAGE_PATH;

pub struct BaseClient {
    inner: reqwest::Client,
    base_url: String,
}

pub trait ChatClient: Send + Sync + 'static {
    fn chat<'a>(&'a self, input: ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>>;
}

impl ChatClient for BaseClient {
    fn chat<'a>(&'a self, input: ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>> {
        async move {
            let mut backoff = ExponentialBackoff::default();
            loop {
                let resp =
                    self.inner.post(format!("{}{}", self.base_url, input.endpoint.as_uri()))
                        .json(&input.body)
                        .send().await?
                        .bytes().await?;
                match serde_json::from_slice::<ChatResponseResult>(&resp) {
                    Ok(ChatResponseResult::ChatResponse(x)) => return Ok(x),
                    Ok(ChatResponseResult::ChatResponseError(x)) => {
                        eprintln!("{:?}", resp);
                        if x.error.typ == "server_error" {
                            if let Some(backoff) = backoff.next_backoff() {
                                sleep(backoff).await;
                                continue;
                            }
                        }
                        return Err(x.into());
                    }
                    Err(e) => {
                        eprintln!("{:?}", resp);
                        return Err(e.into());
                    }
                }
            }
        }.boxed()
    }
}

impl BaseClient {
    pub async fn new() -> anyhow::Result<Self> {
        let api_key = home::home_dir().unwrap().join(".config/chatgpt_apikey.txt");
        let api_key = fs::read_to_string(api_key).await?;
        let api_key = api_key.trim();
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_bytes(format!("Bearer {api_key}").as_bytes())?,
        );
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;
        Ok(BaseClient {
            inner: client,
            base_url: "https://api.openai.com".to_string(),
        })
    }
}

pub struct CacheClient {
    inner: Arc<dyn ChatClient>,
    cache: Box<KeyValueFile<ChatRequest, ChatResponse>>,
}

impl ChatClient for CacheClient {
    fn chat<'a>(&'a self, input: ChatRequest) -> BoxFuture<'a, anyhow::Result<ChatResponse>> {
        self.chat_impl(input).boxed()
    }
}

impl CacheClient {
    pub async fn new(x: Arc<dyn ChatClient>, path: &Path) -> io::Result<(Self, KeyValueFileCleanup)> {
        let (kvf, cleanup) = KeyValueFile::new(path).await?;
        Ok((CacheClient { inner: x, cache: Box::new(kvf) }, cleanup))
    }
    async fn chat_impl(&self, input: ChatRequest) -> anyhow::Result<ChatResponse> {
        let inner = self.inner.clone();
        Ok((*self.cache.get_or_init(input.clone(), async move {
            inner.chat(input).await
        }).await?).clone())
    }
}

#[tokio::test]
async fn test_base_client() -> anyhow::Result<()> {
    let base_client = BaseClient::new().await?;
    let response = base_client.chat(ChatRequest {
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
    Ok(())
}

#[tokio::test]
async fn test_cache_client() -> anyhow::Result<()> {
    let (cache_client, cleanup) =
        CacheClient::new(Arc::new(BaseClient::new().await?),
                         &PACKAGE_PATH.join("build/chat_cache.txt")).await?;
    let response = cache_client.chat(ChatRequest {
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