#![allow(unused_variables, unused_mut)]

use crate::lemma::{Lemma, LEMMA};
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
use crate::llm::new_client;
use crate::llm::rpcs::{AnswerRequest, ClueRequest};
// use crate::gpt::types::{ChatMessage, ChatRequest, ChatRequestBody, ChatRole, Endpoint, FinishReason, Model};
use crate::ontology::{Ontology, ONTOLOGY};
use crate::PACKAGE_PATH;

use crate::puzzle::Puzzle;
use crate::string::LetterString;
use crate::subseq::longest_subsequence;
use crate::util::interrupt::{channel, CleanupSender};
use crate::util::lazy_async::CloneError;

static MODEL: &str = "llama3.2:3b";
// static MODEL: &str = "llama3.3:70b";
pub struct ClueClient {
    client: Arc<dyn ChatClient>,
    ontology: Arc<Ontology>,
    lemma: Arc<Lemma>,
}

impl ClueClient {
    pub async fn new(cleanup: CleanupSender) -> anyhow::Result<Self> {
        let client = new_client(cleanup).await?;
        Ok(ClueClient {
            client,
            ontology: ONTOLOGY.get().await.clone_error_static()?.clone(),
            lemma: LEMMA.get().await.clone_error_static()?.clone(),
        })
    }
    pub fn score(&self, word: &str, clue: &str) -> Option<NotNan<f64>> {
        let word_letters = LetterString::from_str(word);
        let clue_letters = LetterString::from_str(clue);
        let mut is_banned = false;

        for banned in self
            .lemma
            .alternates(word)
            .iter()
            .chain(self.lemma.canonicals(word).iter())
            .chain(self.ontology.get_conflicts(word).iter())
        {
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
            None
        } else {
            Some(-(NotNan::new(longest_subsequence(&word_letters, &clue_letters) as f64).unwrap()))
        }
    }
    pub async fn create_clue(&self, answer: &str) -> anyhow::Result<Option<String>> {
        println!("creating clue for `{}`", answer);
        for seed in 0..10 {
            let mut clues = ClueRequest {
                answer: answer.to_string(),
                clue_count: 10,
            }
            .build()?
            .seed(123455454 + seed)
            .send(&*self.client)
            .await?
            .clues;
            let mut clues = clues
                .into_iter()
                .filter_map(|clue| {
                    let score = self.score(&answer, &clue)?;
                    Some((clue, score))
                })
                .collect::<Vec<_>>();
            clues.sort_by_key(|x| -x.1);
            let clues = clues
                .into_iter()
                .map(|(clue, score)| clue)
                .collect::<Vec<_>>();
            println!("    candidate clues {:?}", clues);
            for clue in clues {
                println!("    candidate clue {}", clue);
                let answers = AnswerRequest {
                    clue: clue.clone(),
                    letter_count: answer.len(),
                    answer_count: 10,
                }
                .build()?
                .send(&*self.client)
                .await?
                .answers;
                println!("        candidate answers {:?}", answers);
                if answers
                    .iter()
                    .any(|x| LetterString::from_str(x) == LetterString::from_str(answer))
                {
                    println!("       Done! `{}` <= `{}`", answer, clue);
                    return Ok(Some(clue));
                }
            }
        }
        Ok(None)
    }
}

pub async fn add_chat(pindex: usize, client: &ClueClient) -> anyhow::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    let clues = puzzle
        .clues
        .as_mut()
        .unwrap()
        .iter_mut()
        .map(|clue| async move {
            clue.clue = client.create_clue(&clue.answer).await?;
            // println!("{:?}: {:?}", clue.answer, clue.clue);
            anyhow::Result::<_>::Ok(())
        });
    for clue in clues {
        clue.await?;
    }
    // try_join_all(clues).await?;
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
    let (tx, mut rx) = channel();
    let client = ClueClient::new(tx).await?;
    {
        let start = Instant::now();
        for word in &[
            // "extant", "netball",
            // "nathan",
            // "andrew", "john", "hindwings"
            "dudley",
        ] {
            let clues = client.create_clue(word).await?;
            println!("{:?}", clues);
        }
    }
    mem::drop(client);
    rx.cleanup().await?;
    Ok(())
}
