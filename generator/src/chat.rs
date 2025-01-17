#![allow(unused_variables, unused_mut)]

use std::{fs, future, mem};
use std::sync::Arc;
use std::time::Instant;
use futures::future::{join_all, try_join_all};
use itertools::Itertools;
use ordered_float::NotNan;
use tokio::{io, spawn};
use acrostic_core::letter::Letter;
use crate::gpt::cache_client::CacheClient;
use crate::gpt::chat_client::{BaseClient, ChatClient};
use crate::gpt::key_value_file::KeyValueFileCleanup;
use crate::gpt::new_client;
use crate::gpt::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatRole, Endpoint, FinishReason, Model};
use crate::ontology::{Ontology, ONTOLOGY};
use crate::PACKAGE_PATH;

use crate::puzzle::Puzzle;
use crate::string::LetterString;
use crate::subseq::longest_subsequence;
use crate::util::lazy_async::CloneError;

pub struct ClueClient {
    client: Arc<dyn ChatClient>,
    ontology: Arc<Ontology>,
}


impl ClueClient {
    pub async fn new() -> anyhow::Result<(Self, KeyValueFileCleanup)> {
        let (client, cleanup) = new_client().await?;
        Ok((ClueClient {
            client,
            ontology: ONTOLOGY.get().await.clone_error_static()?.clone(),
        }, cleanup))
    }
    pub fn score(&self, word: &str, clue: &str) -> NotNan<f64> {
        let word_letters = LetterString::from_str(word);
        let clue_letters = LetterString::from_str(clue);
        let mut is_banned = false;
        for banned in self.ontology.get_conflicts(word) {
            let banned_letters = LetterString::from_str(&banned);
            if banned_letters.len() >= 3 {
                if clue_letters.windows(banned_letters.len()).any(|x| x == &*banned_letters) {
                    // eprintln!("clue {:?} contains {:?} which is banned for {:?}", clue, banned, word);
                    is_banned = true;
                }
            }
        }
        if is_banned {
            NotNan::new(-f64::INFINITY).unwrap()
        } else {
            -(NotNan::new(longest_subsequence(&word_letters, &clue_letters) as f64).unwrap())
        }
    }
    pub async fn create_clue(&self, word: &str) -> anyhow::Result<Option<String>> {
        let mut clues = self.create_clue_list(word).await?;
        clues.sort_by_cached_key(|x| self.score(&word, x));
        // println!("{:#?}", clues);
        let clue = if let Some(clue) = clues.pop() { clue } else { return Ok(None); };
        let clue = &clue;
        let clue = clue.strip_prefix("Answer: ").unwrap_or(clue);
        let clue = clue.strip_prefix("A: ").unwrap_or(clue);
        let clue = clue.strip_prefix("Possible clue: ").unwrap_or(clue);
        Ok(Some(clue.to_string()))
    }
    pub async fn create_clue_list(&self, word: &str) -> anyhow::Result<Vec<String>> {
        let m1 = ChatMessage {
            role: ChatRole::System,
            content: "
You are a crossword clue generator that follows precise rules:
* The clue is short and succinct with minimal detail.
* The clue agrees with the input in tense, part of speech, and plurality.
* The clue and input do not share an etymology.
* You clue names by describing a famous person with that name.
Examples:
Q: dog
A: A furry pet
Q: difficult
A: Hard to accomplish.
Q: london
A: Largest city in England.
Q: nathan
A: Actor known for portraying Mal on Firefly.
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
            n: Some(10),
            max_tokens: Some(15),
            temperature: Some(NotNan::new(1.0).unwrap()),
            ..Default::default()
        };
        let request = ChatRequest { endpoint: Endpoint::Chat, body };
        let response = self.client.chat(&request).await?;
        Ok(response.choices.iter()
            .filter(|x| x.finish_reason.unwrap() == FinishReason::Stop)
            .map(|x| x.message.content.to_string()).collect())
    }
    pub async fn solve_clue(&self, clue: &str) -> anyhow::Result<()> {
        todo!()
    }
}

pub async fn add_chat(pindex: usize, client: &ClueClient) -> anyhow::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    try_join_all(puzzle.clues.as_mut().unwrap().iter_mut().map(|clue| async move {
        clue.clue = client.create_clue(&clue.answer).await?;
        // println!("{:?}: {:?}", clue.answer, clue.clue);
        anyhow::Result::<_>::Ok(())
    })).await?;
    if puzzle.clues.as_ref().unwrap().iter().all(|x| x.clue.is_some()) {
        puzzle.write(pindex, "stage3.json").await?;
    }
    Ok(())
}

#[tokio::test]
async fn test_clue_client() -> anyhow::Result<()> {
    let (client, cleanup) = ClueClient::new().await?;
    {
        let start = Instant::now();
        for word in &["extant", "netball", "nathan", "andrew", "john", "hindwings"] {
            let clues = client.create_clue(word).await?;
            println!("{:?}", clues);
        }
    }
    mem::drop(client);
    cleanup.cleanup().await?;
    Ok(())
}

