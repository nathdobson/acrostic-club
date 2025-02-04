use crate::llm::cache_client::CacheClient;
use crate::llm::chat_client::{BaseClient, ChatClient};
use crate::llm::rate_limit_client::RateLimitClient;
use crate::llm::retry_client::RetryClient;
use crate::util::clock::Clock;
use crate::util::interrupt::CleanupSender;
use crate::util::rate_limit::RateLimit;
use crate::PACKAGE_PATH;
use anyhow::anyhow;
use ollama_rs::error::{OllamaError, ToolCallError};
use std::io;
use std::sync::Arc;

pub mod cache_client;
pub mod chat_client;
pub mod key_value_file;
pub mod rate_limit_client;
pub mod retry_client;
pub mod rpcs;

pub const MODEL_NAME: &'static str = "phi4";
pub async fn new_client(cleanup: CleanupSender) -> anyhow::Result<Arc<dyn ChatClient>> {
    let client = BaseClient::new().await?;
    // let rate = RateLimit::new(Clock::Real, 50, 1.0);
    // let client = RateLimitClient::new(client, rate);
    // let client = RetryClient::new_exponential(client);
    let client =
        CacheClient::new(client, &PACKAGE_PATH.join("build/chat_cache.txt"), cleanup).await?;
    Ok(client)
}

pub fn ollama_to_anyhow(ollama: OllamaError) -> anyhow::Error {
    match ollama {
        OllamaError::ToolCallError(e) => match e {
            ToolCallError::UnknownToolName => anyhow!("UnknownToolName"),
            ToolCallError::InvalidToolArguments(e) => anyhow::Error::from(e),
            ToolCallError::InternalToolError(e) => anyhow!("InternalToolError {}", e),
        },
        OllamaError::JsonError(e) => anyhow::Error::from(e),
        OllamaError::ReqwestError(e) => anyhow::Error::from(e),
        OllamaError::InternalError(e) => anyhow!("InternalError {}", e.message),
        OllamaError::Other(e) => anyhow!("Other {}", e),
    }
}
