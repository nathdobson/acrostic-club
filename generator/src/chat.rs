#![allow(unused_variables, unused_mut)]

use std::default::default;
use std::fs;
use std::sync::Arc;
use std::time::Instant;
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
* Your output is just the clue.
* The clue is short and succinct.
* The clue agrees with the input in tense, part of speech, and plurality.
* The clue and input do not share an etymology.
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
        println!("{:#?}", response);
        Ok(response.choices.iter()
            .filter(|x| x.finish_reason.unwrap() == FinishReason::Stop)
            .map(|x| x.message.content.to_string()).next())
    }
    pub async fn solve_clue(&self, clue: &str) -> anyhow::Result<()> {
        todo!()
    }
}


pub async fn add_chat(pindex: usize, client: &ClueClient) -> anyhow::Result<()> {
//     let api_key = home::home_dir().unwrap().join(".config/chatgpt_apikey.txt");
//     let api_key = fs::read_to_string(api_key).unwrap();
//     let api_key = api_key.trim();
//     let base_url = "https://api.openai.com";
//     let client = ChatGPTClient::new(api_key, base_url);
//
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
//     for clue in puzzle.clues.unwrap().iter_mut() {
//         client.create_clue(&clue.answer).await?;
//     }
    let mut chat_input = ChatRequestBody {
        model: Model::GPT_3_5_TURBO,
        messages: vec![ChatMessage {
            role: ChatRole::System,
            content: "
You are a crossword clue generator that follows precise rules:
1. You generate one clue for each input word.
2. Clues are at most five words long.
3. Clues are short and succinct.
4. Clues agree with the input in tense, part of speech, and plurality.
5. Clues and inputs do not share an etymology.
"
                .to_string(),
        }],
        ..Default::default()
    };
    chat_input.messages.push(ChatMessage {
        role: ChatRole::User,
        content: puzzle
            .clues
            .as_ref()
            .unwrap()
            .iter()
            .map(|x| &x.answer)
            .join(" "),
    });
    println!("{:#?}", chat_input);
    let response = client.client.chat(ChatRequest {
        endpoint: Endpoint::Chat,
        body: chat_input,
    }).await.unwrap();
    println!("{:#?}", response);
    puzzle.chat = Some(response.choices[0].message.content.to_string());

    puzzle.write(pindex, "stage3.json").await?;
    Ok(())
}

#[tokio::test]
async fn test_clue_client() -> anyhow::Result<()> {
    let mut client = ClueClient::new().await?;
    let start = Instant::now();
    for word in &["roadways"] {
        let clues = client.create_clue(word).await?;
        println!("{:?}", clues);
    }
    client.shutdown().await?;
    // for x in clues.into_iter().map(|clue| spawn({
    //     let client = client.clone();
    //     async move {
    //         client.solve_clue(&clue).await
    //     }
    // })) {
    //     x.await??;
    // };
    Ok(())
}