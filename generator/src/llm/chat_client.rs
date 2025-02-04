use crate::llm::key_value_file::KeyValueFile;
use crate::llm::ollama_to_anyhow;
use anyhow::anyhow;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::FutureExt;
use ollama_rs::error::{OllamaError, ToolCallError};
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::ChatMessageResponse;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::generation::parameters::{FormatType, JsonStructure};
use ollama_rs::Ollama;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use serde::Serialize;
use std::any::Any;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::{io, mem};
use tokio::fs;
use tokio::time::sleep;
// use crate::llm::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatResponse, ChatResponseResult, ChatRole, Endpoint, Model};
use crate::PACKAGE_PATH;

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
