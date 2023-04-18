#![allow(unused_variables, unused_mut)]

use std::fs;

use chat_gpt_lib_rs::{ChatGPTClient, ChatInput, Message, Model, Role};
use itertools::Itertools;
use tokio::io;

use crate::puzzle::Puzzle;

pub async fn add_chat(pindex: usize) -> io::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    let api_key = home::home_dir().unwrap().join(".config/chatgpt_apikey.txt");
    let api_key = fs::read_to_string(api_key).unwrap();
    let api_key = api_key.trim();
    let base_url = "https://api.openai.com";
    let client = ChatGPTClient::new(api_key, base_url);
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
