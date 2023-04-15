#[allow(unused_imports)]
use std::collections::HashMap;
use std::io;

use acrostic::puzzle::Puzzle;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut puzzle = Puzzle::read("stage3.json")?;
    let table: Vec<&str> = puzzle
        .chat
        .as_ref()
        .unwrap()
        .split("\n")
        .filter(|x| !x.is_empty())
        .collect();
    for clue in puzzle.clues.as_mut().unwrap() {
        let  entry = table
            .iter()
            .filter_map(|x| {
                x.to_ascii_lowercase()
                    .strip_prefix(&clue.answer.to_ascii_lowercase())
                    .map(|x| x.to_string())
                    .filter(|x| !x.chars().next().unwrap().is_alphabetic())
            })
            .next()
            .unwrap_or_else(|| panic!("Can't find {:?}", clue.answer));
        let mut entry = &*entry;
        entry = entry.strip_prefix(" - ").unwrap_or(entry);
        entry = entry.strip_prefix(": ").unwrap_or(entry);
        clue.clue = Some(entry.to_string());
    }
    puzzle.write("stage4.json")?;
    Ok(())
}
