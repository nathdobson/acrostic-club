#![allow(unused_variables, unused_mut)]

use acrostic_core::letter::Letter;
use futures::future::{join_all, try_join_all};
use itertools::Itertools;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::generation::parameters::{FormatType, JsonSchema, JsonStructure};
use ordered_float::NotNan;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use std::{fs, future, mem};
use tokio::{io, spawn};
// use crate::gpt::cache_client::CacheClient;
use crate::llm::chat_client::{BaseClient, ChatClient};
use crate::llm::key_value_file::KeyValueFileCleanup;
use crate::llm::new_client;
// use crate::gpt::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatRole, Endpoint, FinishReason, Model};
use crate::ontology::{Ontology, ONTOLOGY};
use crate::PACKAGE_PATH;

use crate::puzzle::Puzzle;
use crate::string::LetterString;
use crate::subseq::longest_subsequence;
use crate::util::lazy_async::CloneError;

static MODEL: &str = "llama3.2:3b";
// static MODEL: &str = "llama3.3:70b";
pub struct ClueClient {
    client: Arc<dyn ChatClient>,
    ontology: Arc<Ontology>,
}

#[derive(JsonSchema, Deserialize, Debug)]
struct ClueResponse {
    answer: String,
    clues: Vec<String>,
}

#[derive(JsonSchema, Deserialize, Debug)]
struct AnswerResponse {
    answers: Vec<String>,
}

impl ClueClient {
    pub async fn new() -> anyhow::Result<(Self, KeyValueFileCleanup)> {
        let (client, cleanup) = new_client().await?;
        Ok((
            ClueClient {
                client,
                ontology: ONTOLOGY.get().await.clone_error_static()?.clone(),
            },
            cleanup,
        ))
    }
    pub fn score(&self, word: &str, clue: &str) -> NotNan<f64> {
        let word_letters = LetterString::from_str(word);
        let clue_letters = LetterString::from_str(clue);
        let mut is_banned = false;
        for banned in self.ontology.get_conflicts(word) {
            let banned_letters = LetterString::from_str(&banned);
            if banned_letters.len() >= 3 {
                if clue_letters
                    .windows(banned_letters.len())
                    .any(|x| x == &*banned_letters)
                {
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
        let mut solved = false;
        for clue in &clues {
            if self.solve_clue(clue, word.len(), word).await? {
                solved = true;
                break;
            }
        }
        if !solved {
            return Ok(None);
        }
        clues.sort_by_cached_key(|x| self.score(&word, x));
        let clue = if let Some(clue) = clues.pop() {
            clue
        } else {
            return Ok(None);
        };
        let clue = &clue;
        let clue = clue.strip_prefix("Answer: ").unwrap_or(clue);
        let clue = clue.strip_prefix("A: ").unwrap_or(clue);
        let clue = clue.strip_prefix("Possible clue: ").unwrap_or(clue);
        Ok(Some(clue.to_string()))
    }
    pub async fn create_clue_list(&self, word: &str) -> anyhow::Result<Vec<String>> {
        println!("Creating a clue list for {}", word);
        let response = self
            .client
            .chat(
                &GenerationRequest::new(
                    MODEL.to_string(),
                    format!("Create a collection of ten crossword clues for '{}'.", word),
                )
                .options(GenerationOptions::default().seed(123542323))
                .format(FormatType::StructuredJson(
                    JsonStructure::new::<ClueResponse>(),
                )),
            )
            .await?;
        let response = serde_json::from_str::<ClueResponse>(&response.response)?;
        println!("clue for {} is {:#?}", word, response);
        Ok(response.clues)
    }
    pub async fn solve_clue(
        &self,
        clue: &str,
        len: usize,
        actual_answer: &str,
    ) -> anyhow::Result<bool> {
        println!("Trying to solve '{}'", clue);
        for seed in 0..10 {
            let response = self
                .client
                .chat(
                    &GenerationRequest::new(
                        MODEL.to_string(),
                        format!(
                            "Provide 5 possible {}-letter answers to the crossword clue '{}'.",
                            len, clue
                        ),
                    )
                    .options(GenerationOptions::default().seed(23443 + seed).num_predict(100))
                    .format(FormatType::StructuredJson(JsonStructure::new::<
                        AnswerResponse,
                    >())),
                )
                .await?;
            let response = serde_json::from_str::<AnswerResponse>(&response.response)?;
            println!("response={:?}", response);
            for answer in &response.answers {
                if answer == actual_answer {
                    return Ok(true);
                }
            }
        }
        Ok(false)
        // let response = serde_json::from_str::<ClueResponse>(&response.response)?;
        // println!("answers are {:#?}", response);
        // Ok(response.clues)
    }
}

pub async fn add_chat(pindex: usize, client: &ClueClient) -> anyhow::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    try_join_all(
        puzzle
            .clues
            .as_mut()
            .unwrap()
            .iter_mut()
            .map(|clue| async move {
                clue.clue = client.create_clue(&clue.answer).await?;
                // println!("{:?}: {:?}", clue.answer, clue.clue);
                anyhow::Result::<_>::Ok(())
            }),
    )
    .await?;
    if puzzle
        .clues
        .as_ref()
        .unwrap()
        .iter()
        .all(|x| x.clue.is_some())
    {
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
