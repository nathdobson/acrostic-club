use crate::llm::key_value_file::{KeyValueFile, KeyValueFileCleanup};
use anyhow::anyhow;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::FutureExt;
use ollama_rs::error::{OllamaError, ToolCallError};
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
    fn chat<'a>(
        &'a self,
        input: &'a GenerationRequest,
    ) -> BoxFuture<'a, anyhow::Result<GenerationResponse>>;
}

impl ChatClient for BaseClient {
    fn chat<'a>(
        &'a self,
        input: &'a GenerationRequest,
    ) -> BoxFuture<'a, anyhow::Result<GenerationResponse>> {
        async move {
            Ok(self
                .inner
                .generate(input.clone())
                .await
                .map_err(|e| match e {
                    OllamaError::ToolCallError(e) => match e {
                        ToolCallError::UnknownToolName => anyhow!("UnknownToolName"),
                        ToolCallError::InvalidToolArguments(e) => anyhow::Error::from(e),
                        ToolCallError::InternalToolError(e) => anyhow!("InternalToolError {}", e),
                    },
                    OllamaError::JsonError(e) => anyhow::Error::from(e),
                    OllamaError::ReqwestError(e) => anyhow::Error::from(e),
                    OllamaError::InternalError(e) => anyhow!("InternalError {}", e.message),
                    OllamaError::Other(e) => anyhow!("Other {}", e),
                })?)
            // let resp = self
            //     .inner
            //     .post(format!("{}{}", self.base_url, input.endpoint.as_uri()))
            //     .json(&input.body)
            //     .send()
            //     .await?
            //     .bytes()
            //     .await?;
            // match serde_json::from_slice::<ChatResponseResult>(&resp) {
            //     Ok(ChatResponseResult::ChatResponse(x)) => return Ok(x),
            //     Ok(ChatResponseResult::ChatResponseError(x)) => {
            //         println!("Error: {:?}", x.error.typ);
            //         return Err(x.into());
            //     }
            //     Err(e) => {
            //         eprintln!("{:?}", resp);
            //         return Err(e.into());
            //     }
            // }
            // }
        }
        .boxed()
    }
}

impl BaseClient {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let api_key = home::home_dir().unwrap().join(".config/chatgpt_apikey.txt");
        let api_key = fs::read_to_string(api_key).await?;
        let api_key = api_key.trim();
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_bytes(format!("Bearer {api_key}").as_bytes())?,
        );
        let client = Ollama::default();
        Ok(Arc::new(BaseClient { inner: client }))
    }
}

#[tokio::test]
async fn test_base_client() -> anyhow::Result<()> {
    let base_client = BaseClient::new().await?;
    let response = base_client
        .chat(
            &GenerationRequest::new(
                "llama3.2:3b".to_string(),
                "What does a dog say?".to_string(),
            )
            .options(GenerationOptions::default().seed(123553))
            .format(FormatType::StructuredJson(JsonStructure::new::<String>())),
        )
        .await?;
    assert_eq!(response.response, "\"Drool\"");
    Ok(())
}
