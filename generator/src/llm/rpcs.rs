use crate::llm::chat_client::{BaseClient, ChatClient};
use crate::llm::{ollama_to_anyhow, MODEL_NAME};
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

pub struct RpcBuilder<Req, Resp> {
    req: Req,
    training: Vec<(Req, Resp)>,
    system: String,
    seed: i32,
    model: &'static str,
}

impl<Req: Serialize, Resp: JsonSchema + Serialize + for<'de> Deserialize<'de>>
    RpcBuilder<Req, Resp>
{
    pub fn new(req: Req, system: String) -> anyhow::Result<Self> {
        Ok(RpcBuilder {
            req,
            training: vec![],
            system,
            seed: 123665,
            model: MODEL_NAME,
        })
    }
    pub fn seed(&mut self, seed: i32) -> &mut Self {
        self.seed = seed;
        self
    }
    pub fn train(&mut self, req: Req, resp: Resp) -> &mut Self {
        self.training.push((req, resp));
        self
    }
    pub async fn send(&self, ollama: &dyn ChatClient) -> anyhow::Result<Resp> {
        let mut messages = vec![];
        messages.push(ChatMessage::new(MessageRole::System, self.system.clone()));
        for (req, resp) in &self.training {
            messages.push(ChatMessage::new(
                MessageRole::User,
                serde_json::to_string(req)?,
            ));
            messages.push(ChatMessage::new(
                MessageRole::Assistant,
                serde_json::to_string(resp)?,
            ));
        }
        messages.push(ChatMessage::new(
            MessageRole::User,
            serde_json::to_string(&self.req)?,
        ));
        let resp = ollama
            .send_chat_messages(
                &ChatMessageRequest::new(self.model.to_string(), messages)
                    .options(GenerationOptions::default().seed(self.seed))
                    .format(FormatType::StructuredJson(JsonStructure::new::<Resp>())),
            )
            .await?;
        // .map_err(ollama_to_anyhow)?;
        let resp = serde_json::from_str(&resp.message.content)?;
        Ok(resp)
    }
}

#[derive(Serialize)]
pub struct ClueRequest {
    pub answer: String,
    pub clue_count: usize,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct ClueResponse {
    pub clues: Vec<String>,
}

impl ClueRequest {
    pub fn build(self) -> anyhow::Result<RpcBuilder<ClueRequest, ClueResponse>> {
        let mut rpc = RpcBuilder::new(
            self,
            "You are a crossword clue generator. \
                        You generate several diverse crossword clues for a given answer."
                .to_string(),
        )?;
        rpc.train(
            ClueRequest {
                answer: "dog".to_string(),
                clue_count: 2,
            },
            ClueResponse {
                clues: vec!["Furry pet.".to_string(), "Man's best friend.".to_string()],
            },
        );
        rpc.train(
            ClueRequest {
                answer: "einstein".to_string(),
                clue_count: 3,
            },
            ClueResponse {
                clues: vec![
                    "Albert of physics fame".to_string(),
                    "He postulated E=mc^2.".to_string(),
                    "Eponym of genius".to_string(),
                ],
            },
        );
        rpc.train(
            ClueRequest {
                answer: "pitt".to_string(),
                clue_count: 1,
            },
            ClueResponse {
                clues: vec![
                    "Brad ____ from the silver screen.".to_string(),
                ],
            },
        );
        Ok(rpc)
    }
}

#[derive(Serialize)]
pub struct AnswerRequest {
    pub clue: String,
    pub letter_count: usize,
    pub answer_count: usize,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct AnswerResponse {
    pub answers: Vec<String>,
}

impl AnswerRequest {
    pub fn build(self) -> anyhow::Result<RpcBuilder<AnswerRequest, AnswerResponse>> {
        RpcBuilder::new(
            self,
            "You are a crossword clue solver. You provide several possible answers for a crossword clue. You do not use any forms of the answer word in the clue.".to_string(),
        )
    }
}

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    // let ollama = Ollama::default();
    let client = BaseClient::new().await?;
    let answer = "nathan";
    let clues = ClueRequest {
        answer: answer.to_string(),
        clue_count: 30,
    }
    .build()?
    .send(&*client)
    .await?;
    println!("{:#?}", clues);
    Ok(())
}
