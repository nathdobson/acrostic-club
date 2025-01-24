use crate::gpt::chat_client::ChatClient;
use crate::gpt::{ollama_to_anyhow, FULL_MODEL, TEST_MODEL};
use futures::stream::iter;
use futures::StreamExt;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::{ChatMessage, MessageRole};
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::generation::parameters::{FormatType, JsonSchema, JsonStructure};
use ollama_rs::Ollama;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

pub struct RpcBuilder<Resp> {
    req: String,
    system: String,
    seed: i32,
    model: &'static str,
    phantom: PhantomData<Resp>,
}

impl<Resp: JsonSchema + for<'de> Deserialize<'de>> RpcBuilder<Resp> {
    pub fn new<Req: Serialize>(req: &Req, system: String) -> anyhow::Result<Self> {
        Ok(RpcBuilder {
            req: serde_json::to_string(req)?,
            system,
            seed: 123665,
            model: "gemma2:27b",
            phantom: Default::default(),
        })
    }
    pub fn seed(&mut self, seed: i32) -> &mut Self {
        self.seed = seed;
        self
    }
    pub async fn send(&self, ollama: &Ollama) -> anyhow::Result<Resp> {
        let resp = ollama
            .send_chat_messages(
                ChatMessageRequest::new(
                    self.model.to_string(),
                    vec![
                        ChatMessage::new(MessageRole::System, self.system.clone()),
                        ChatMessage::new(MessageRole::User, self.req.clone()),
                    ],
                )
                .options(GenerationOptions::default().seed(self.seed))
                .format(FormatType::StructuredJson(JsonStructure::new::<Resp>())),
            )
            .await
            .map_err(ollama_to_anyhow)?;
        let resp = serde_json::from_str(&resp.message.content)?;
        Ok(resp)
    }
}

#[derive(Serialize)]
pub struct ClueRequest {
    pub answer: String,
    pub clue_count: usize,
}

#[derive(JsonSchema, Deserialize, Debug)]
pub struct ClueResponse {
    // answer: String,
    clues: Vec<String>,
}

impl ClueRequest {
    pub fn build(self) -> anyhow::Result<RpcBuilder<ClueResponse>> {
        RpcBuilder::new(&self, "You are a crossword clue generator.".to_string())
    }
}

#[derive(Serialize)]
pub struct AnswerRequest {
    pub clue: String,
    pub letter_count: usize,
    pub answer_count: usize,
}

#[derive(JsonSchema, Deserialize, Debug)]
pub struct AnswerResponse {
    answers: Vec<String>,
}

impl AnswerRequest {
    pub fn build(self) -> anyhow::Result<RpcBuilder<AnswerResponse>> {
        RpcBuilder::new(
            &self,
            "You are a crossword clue solver. You provide several possible answers for a crossword clue.".to_string(),
        )
    }
}

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let ollama = Ollama::default();
    let answer = "extant";
    let clues = ClueRequest {
        answer: answer.to_string(),
        clue_count: 10,
    }
    .build()?
    .send(&ollama)
    .await?;
    println!("{:?}", clues);
    Ok(())
}
