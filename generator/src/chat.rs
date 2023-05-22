#![allow(unused_variables, unused_mut)]

use std::default::default;
use std::{fs, future};
use std::sync::Arc;
use std::time::Instant;
use futures::future::{join_all, try_join_all};
use itertools::Itertools;
use ordered_float::NotNan;
use tokio::{io, spawn};
use crate::chat_client::{BaseClient, CacheClient, ChatClient};
use crate::gpt::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatRole, Endpoint, FinishReason, Model};
use crate::PACKAGE_PATH;

use crate::puzzle::Puzzle;

pub struct ClueClient {
    client: Box<dyn ChatClient>,
}

impl ClueClient {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(ClueClient {
            client: Box::new(
                CacheClient::new(Box::new(BaseClient::new().await?),
                                 &PACKAGE_PATH.join("build/chat_cache.txt")).await?)
        })
    }
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.client.shutdown().await
    }
    pub async fn create_clue(&self, word: &str) -> anyhow::Result<Option<String>> {
        let m1 = ChatMessage {
            role: ChatRole::System,
            content: "
You are a crossword clue generator that follows precise rules:
* The clue is short and succinct.
* The clue agrees with the input in tense, part of speech, and plurality.
* The clue and input do not share an etymology.
Examples:
Q: dog
A: A furry pet
Q: difficult
A: Hard to accomplish.
Q: london
A: Largest city in England.
"
                .to_string(),
        };
        let m2 = ChatMessage {
            role: ChatRole::User,
            content: format!("Generate a clue for '{}'", word),
        };
        let body = ChatRequestBody {
            model: Model::GPT_3_5_TURBO,
            messages: vec![m1, m2],
            n: Some(5),
            max_tokens: Some(15),
            temperature: Some(NotNan::new(1.0).unwrap()),
            ..Default::default()
        };
        let request = ChatRequest { endpoint: Endpoint::Chat, body };
        let response = self.client.chat(request).await?;
        Ok(response.choices.iter()
            .filter(|x| x.finish_reason.unwrap() == FinishReason::Stop)
            .map(|x| x.message.content.to_string()).next())
    }
    pub async fn solve_clue(&self, clue: &str) -> anyhow::Result<()> {
        todo!()
    }
}

pub async fn add_chat(pindex: usize, client: &ClueClient) -> anyhow::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    try_join_all(puzzle.clues.as_mut().unwrap().iter_mut().map(|clue| async move {
        clue.clue = client.create_clue(&clue.answer).await?;
        println!("{:?}: {:?}", clue.answer, clue.clue);
        anyhow::Result::<_>::Ok(())
    })).await?;
    if puzzle.clues.as_ref().unwrap().iter().all(|x| x.clue.is_some()) {
        puzzle.write(pindex, "stage3.json").await?;
    }
    Ok(())
}

#[tokio::test]
async fn test_clue_client() -> anyhow::Result<()> {
    let mut client = ClueClient::new().await?;
    let start = Instant::now();
    for word in &["extant", "netball"] {
        let clues = client.create_clue(word).await?;
        println!("{:?}", clues);
    }
    client.shutdown().await?;
    Ok(())
}