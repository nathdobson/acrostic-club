#[allow(unused_imports)]
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;

use crate::puzzle::Puzzle;

pub async fn add_clues(pindex: usize) -> io::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage3.json").await?;
    let table: Vec<&str> = puzzle
        .chat
        .as_ref()
        .unwrap()
        .split("\n")
        .filter(|x| !x.is_empty())
        .collect();
    for clue in puzzle.clues.as_mut().unwrap() {
        let entry = table
            .iter()
            .filter_map(|x| {
                x.to_ascii_lowercase()
                    .strip_prefix(&clue.answer.to_ascii_lowercase())
                    .map(|x| x.to_string())
                    .filter(|x| !x.chars().next().unwrap().is_alphabetic())
            })
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, format!("Can't find {:?}", clue.answer)))?;
        let mut entry = &*entry;
        entry = entry.strip_prefix(" - ").unwrap_or(entry);
        entry = entry.strip_prefix(": ").unwrap_or(entry);
        clue.clue = Some(entry.to_string());
    }
    puzzle.write(pindex, "stage4.json").await?;
    Ok(())
}
