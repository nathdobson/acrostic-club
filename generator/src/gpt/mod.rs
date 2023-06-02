use std::io;
use std::sync::Arc;
use crate::gpt::cache_client::CacheClient;
use crate::gpt::chat_client::{BaseClient, ChatClient};
use crate::gpt::key_value_file::KeyValueFileCleanup;
use crate::gpt::rate_limit_client::RateLimitClient;
use crate::gpt::retry_client::RetryClient;
use crate::PACKAGE_PATH;
use crate::util::rate_limit::RateLimit;
use crate::util::clock::Clock;

pub mod types;
pub mod chat_client;
pub mod key_value_file;
pub mod cache_client;
pub mod retry_client;
pub mod rate_limit_client;

pub async fn new_client() -> anyhow::Result<(Arc<dyn ChatClient>, KeyValueFileCleanup)> {
    let client = BaseClient::new().await?;
    let rate = RateLimit::new(Clock::Real, 50, 1.0);
    let client = RateLimitClient::new(client, rate);
    let client = RetryClient::new_exponential(client);
    let (client, cleanup) =
        CacheClient::new(client,
                         &PACKAGE_PATH.join("build/chat_cache.txt")).await?;
    Ok((client, cleanup))
}