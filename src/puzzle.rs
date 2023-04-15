use std::{fs, io};
use chat_gpt_lib_rs::ChatResponse;

use serde::{Deserialize, Serialize};

use crate::{Letter, PACKAGE_PATH};

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
    pub fn read(stage: &str) -> io::Result<Puzzle> {
        let input = fs::read_to_string(&PACKAGE_PATH.join("puzzle").join(stage))?;
        Ok(serde_json::from_str(&input)?)
    }
    pub fn write(&self, stage: &str) -> io::Result<()> {
        fs::write(
            &PACKAGE_PATH.join("puzzle").join(stage),
            &serde_json::to_string_pretty(self).unwrap(),
        )?;
        Ok(())
    }
}
