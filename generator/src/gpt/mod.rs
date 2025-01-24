use crate::gpt::cache_client::CacheClient;
use crate::gpt::chat_client::{BaseClient, ChatClient};
use crate::gpt::key_value_file::KeyValueFileCleanup;
use crate::gpt::rate_limit_client::RateLimitClient;
use crate::gpt::retry_client::RetryClient;
use crate::util::clock::Clock;
use crate::util::rate_limit::RateLimit;
use crate::PACKAGE_PATH;
use ollama_rs::error::{OllamaError, ToolCallError};
use std::io;
use std::sync::Arc;
use anyhow::anyhow;

pub mod cache_client;
pub mod chat_client;
mod json;
pub mod key_value_file;
pub mod rate_limit_client;
pub mod retry_client;
pub mod types;

pub const TEST_MODEL: &'static str = "llama3.2:3b";
pub const FULL_MODEL: &'static str = "llama3.3:70b";

pub async fn new_client() -> anyhow::Result<(Arc<dyn ChatClient>, KeyValueFileCleanup)> {
    let client = BaseClient::new().await?;
    let rate = RateLimit::new(Clock::Real, 50, 1.0);
    let client = RateLimitClient::new(client, rate);
    let client = RetryClient::new_exponential(client);
    let (client, cleanup) =
        CacheClient::new(client, &PACKAGE_PATH.join("build/chat_cache.txt")).await?;
    Ok((client, cleanup))
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
