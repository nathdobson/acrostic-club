use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::Deserialize;
use chrono::serde::ts_seconds;

use ordered_float::NotNan;

// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatRequest {
//     pub endpoint: Endpoint,
//     pub body: ChatRequestBody,
// }
//
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug, Copy)]
// pub enum Endpoint {
//     Chat
// }
//
// impl Endpoint {
//     pub fn as_uri(self) -> &'static str {
//         match self {
//             Endpoint::Chat => "/v1/chat/completions",
//         }
//     }
// }
//
// #[allow(non_camel_case_types)]
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub enum Model {
//     #[serde(rename = "gpt-4")]
//     GPT_4,
//     #[serde(rename = "gpt-4-0314")]
//     GPT_4_0314,
//     #[serde(rename = "gpt-4-32k")]
//     GPT_4_32K,
//     #[serde(rename = "gpt-4-32k-0314")]
//     GPT_4_32K_0314,
//     #[serde(rename = "gpt-3.5-turbo")]
//     GPT_3_5_TURBO,
//     #[serde(rename = "gpt-3.5-turbo-0301")]
//     GPT_3_5_TURBO_0301,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatRequestBody {
//     pub model: Model,
//     pub messages: Vec<ChatMessage>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub temperature: Option<NotNan<f64>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub n: Option<usize>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub max_tokens: Option<usize>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub presence_penalty: Option<NotNan<f64>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub frequency_penalty: Option<NotNan<f64>>,
// }
//
// impl Default for ChatRequestBody {
//     fn default() -> Self {
//         ChatRequestBody {
//             model: Model::GPT_3_5_TURBO,
//             messages: vec![],
//             temperature: None,
//             n: None,
//             max_tokens: None,
//             presence_penalty: None,
//             frequency_penalty: None,
//         }
//     }
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub enum ChatRole {
//     #[serde(rename = "system")]
//     System,
//     #[serde(rename = "user")]
//     User,
//     #[serde(rename = "assistant")]
//     Assistant,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatMessage {
//     pub role: ChatRole,
//     pub content: String,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug, Copy)]
// pub enum FinishReason {
//     #[serde(rename = "length")]
//     Length,
//     #[serde(rename = "stop")]
//     Stop,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatChoice {
//     pub index: usize,
//     pub message: ChatMessage,
//     pub finish_reason: Option<FinishReason>,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatResponse {
//     pub id: String,
//     pub object: String,
//     #[serde(with = "ts_seconds")]
//     pub created: DateTime<Utc>,
//     pub choices: Vec<ChatChoice>,
//     pub usage: ChatUsage,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatResponseErrorInner {
//     pub message: String,
//     #[serde(rename = "type")]
//     pub typ: String,
//     pub param: Option<String>,
//     pub code: Option<String>,
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatResponseError {
//     pub error: ChatResponseErrorInner,
// }
//
// impl Display for ChatResponseError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         Debug::fmt(self, f)
//     }
// }
//
// impl Error for ChatResponseError {}
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// #[serde(untagged)]
// pub enum ChatResponseResult {
//     ChatResponse(ChatResponse),
//     ChatResponseError(ChatResponseError),
// }
//
// #[derive(Serialize, Deserialize, Eq, Ord, PartialOrd, PartialEq, Hash, Clone, Debug)]
// pub struct ChatUsage {
//     pub prompt_tokens: usize,
//     pub completion_tokens: usize,
//     pub total_tokens: usize,
// }
