use std::{fs, io};
use chat_gpt_lib_rs::ChatResponse;

use serde::{Deserialize, Serialize};

use crate::{Letter, PACKAGE_PATH, read_path, read_path_to_string, write_path};

// #[derive(Serialize, Deserialize, Debug)]
// pub struct GivenCell {
//     pub contents: String,
//     pub visible: bool,
// }
//
// #[derive(Serialize, Deserialize, Debug)]
// pub struct EmptyCell {
//     pub letter: Letter,
//     pub content: Option<String>,
//     pub clue_index: Option<usize>,
//     pub clue_offset: Option<usize>,
// }
//
// #[derive(Serialize, Deserialize, Debug)]
// pub enum Cell {
//     Given(GivenCell),
//     Empty(EmptyCell),
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct Clue {
    pub clue: Option<String>,
    pub answer: String,
    pub answer_letters: String,
    pub indices: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Puzzle {
    pub quote: String,
    pub quote_letters: Option<String>,
    pub source: String,
    pub source_letters: Option<String>,
    pub clues: Option<Vec<Clue>>,
    pub chat: Option<String>,
}

impl Puzzle {
    pub async fn read(index: usize, stage: &str) -> io::Result<Puzzle> {
        let input = read_path_to_string(
            &PACKAGE_PATH.join("build/puzzles").join(&format!("{}", index)).join(stage)).await?;
        Ok(serde_json::from_str(&input)?)
    }
    pub async fn write(&self, index: usize, stage: &str) -> io::Result<()> {
        let dir = PACKAGE_PATH.join("build/puzzles").join(&format!("{}", index));
        tokio::fs::create_dir_all(&dir).await?;
        write_path(
            &dir.join(stage),
            &serde_json::to_string_pretty(self).unwrap().as_bytes(),
        ).await?;
        Ok(())
    }
}
