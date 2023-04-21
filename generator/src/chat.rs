#![allow(unused_variables, unused_mut)]

use std::fs;
use std::sync::Arc;
use std::time::Instant;

use chat_gpt_lib_rs::{ChatGPTClient, ChatInput, Message, Model, Role};
use itertools::Itertools;
use tokio::{io, spawn};

use crate::puzzle::Puzzle;

pub struct ClueClient {
    client: ChatGPTClient,
}

impl ClueClient {
    pub fn new() -> Arc<Self> {
        let api_key = home::home_dir().unwrap().join(".config/chatgpt_apikey.txt");
        let api_key = fs::read_to_string(api_key).unwrap();
        let api_key = api_key.trim();
        let base_url = "https://api.openai.com";
        let client = ChatGPTClient::new(api_key, base_url);
        Arc::new(ClueClient { client })
    }
    pub async fn create_clue(&self, word: &str) -> anyhow::Result<Vec<String>> {
        let mut chat_input = ChatInput {
            model: Model::Gpt3_5Turbo,
            messages: vec![],
            n: Some(5),
            max_tokens: Some(15),
            temperature: Some(1.0),
            ..Default::default()
        };
        chat_input.messages.push(Message {
            role: Role::System,
            content: "
You generate a crossword clue for a given input.
The clue is at most five words long.
The clue is short and succinct.
The clue agrees with the input in tense, part of speech, and plurality.
The clue and input do not share an etymology.
"
                .to_string(),
        });
        chat_input.messages.push(Message {
            role: Role::User,
            content: word.to_string(),
        });
        let response = self.client.chat(chat_input).await?;
        println!("{:#?}", response);
        Ok(response.choices.iter().filter(|x| x.finish_reason == "stop").map(|x| x.message.content.to_string()).collect())
    }
    pub async fn solve_clue(&self, clue: &str) -> anyhow::Result<()> {
        let mut chat_input = ChatInput {
            model: Model::Gpt3_5Turbo,
            messages: vec![],
            n: Some(5),
            max_tokens: Some(50),
            temperature: Some(1.5),
            ..Default::default()
        };
        chat_input.messages.push(Message {
            role: Role::System,
            content: "
You solve a crossword clue, outputting ten possible different answers.
The answer agrees with the clue in tense, part of speech, and plurality."
                .to_string(),
        });
        chat_input.messages.push(Message {
            role: Role::User,
            content: clue.to_string(),
        });
        let response = self.client.chat(chat_input).await?;
        println!("{:#?}", clue);
        println!("{:#?}", response);
        Ok(())
    }
}


pub async fn add_chat(pindex: usize) -> io::Result<()> {
    let api_key = home::home_dir().unwrap().join(".config/chatgpt_apikey.txt");
    let api_key = fs::read_to_string(api_key).unwrap();
    let api_key = api_key.trim();
    let base_url = "https://api.openai.com";
    let client = ChatGPTClient::new(api_key, base_url);

    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    let mut chat_input = ChatInput {
        model: Model::Gpt3_5Turbo,
        messages: vec![Message {
            role: Role::System,
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
    chat_input.messages.push(Message {
        role: Role::User,
        content: puzzle
            .clues
            .as_ref()
            .unwrap()
            .iter()
            .map(|x| &x.answer)
            .join(" "),
    });
    println!("{:#?}", chat_input);
    let response = client.chat(chat_input).await.unwrap();
    println!("{:#?}", response);
    puzzle.chat = Some(response.choices[0].message.content.to_string());

    puzzle.write(pindex, "stage3.json").await?;
    Ok(())
}

#[tokio::test]
async fn test_clue_client() -> anyhow::Result<()> {
    let client = ClueClient::new();
    let start = Instant::now();
    let clues = client.create_clue("acerbic").await?;
    println!("{:?}", clues);
    for x in clues.into_iter().map(|clue| spawn({
        let client = client.clone();
        async move {
            client.solve_clue(&clue).await
        }
    })) {
        x.await??;
    };
    Ok(())
}