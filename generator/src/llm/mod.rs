use crate::PACKAGE_PATH;
use crate::llm::cache_client::CacheClient;
use crate::llm::chat_client::{BaseClient, ChatClient};
use crate::util::interrupt::CleanupSender;
use anyhow::anyhow;
use ollama_rs::error::{OllamaError, ToolCallError};
use std::sync::Arc;

pub mod cache_client;
pub mod chat_client;
pub mod key_value_file;
pub mod rpcs;

// The Ollama model that answers a request unless the request sets another.
pub const DEFAULT_MODEL_NAME: &'static str = "phi4";
pub async fn new_client(cleanup: CleanupSender) -> anyhow::Result<Arc<dyn ChatClient>> {
    let client = BaseClient::new().await?;
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
