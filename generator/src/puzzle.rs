use std::io;

use crate::{PACKAGE_PATH, error_with_path, read_path_to_string, write_path};
use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

// The stage file that holds a puzzle's chosen clues. The site publishes every puzzle that has this
// file.
pub const CLUED_PUZZLE_FILE: &str = "stage3.json";

// One puzzle entry: an answer, where its letters sit in the quote, and which clue it shows. A
// field the entry doesn't know makes the entry fail to parse, so a stage file whose entries hold
// clue text in a `clue` field fails loudly rather than reading as entries that name no clue.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Clue {
    // Names this entry's clue among the answer's clues in the clue bank (see `cluebank.rs`).
    // Empty until the `chat` step chooses a clue.
    pub clue_tag: Option<String>,
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
        let path = PACKAGE_PATH
            .join("puzzles")
            .join(&format!("{index}"))
            .join(stage);
        let input = read_path_to_string(&path).await?;
        serde_json::from_str(&input).map_err(error_with_path("parse", &path))
    }
    pub async fn write(&self, index: usize, stage: &str) -> io::Result<()> {
        let dir = PACKAGE_PATH.join("puzzles").join(&format!("{index}"));
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(error_with_path("create", &dir))?;
        write_path(
            &dir.join(stage),
            &serde_json::to_string_pretty(self).unwrap().as_bytes(),
        )
        .await?;
        Ok(())
    }

    // Reads a puzzle's stage file, or returns `None` when the puzzle hasn't reached that stage.
    pub async fn read_if_present(index: usize, stage: &str) -> io::Result<Option<Puzzle>> {
        match Puzzle::read(index, stage).await {
            Ok(puzzle) => Ok(Some(puzzle)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    // Changes the clue tag from `old_tag` to `new_tag` in every entry for `answer`, returning
    // whether any entry changed.
    pub fn retag_clue(&mut self, answer: &str, old_tag: &str, new_tag: &str) -> bool {
        let mut is_changed = false;
        for entry in self.clues.iter_mut().flatten() {
            if entry.answer == answer && entry.clue_tag.as_deref() == Some(old_tag) {
                entry.clue_tag = Some(new_tag.to_string());
                is_changed = true;
            }
        }
        is_changed
    }
}

// Returns the index of every puzzle in the repository's `puzzles` folder, in increasing order.
// Each puzzle's folder is named by its index.
pub async fn puzzle_indices() -> anyhow::Result<Vec<usize>> {
    let puzzles_path = PACKAGE_PATH.join("puzzles");
    let mut directory_entries = tokio::fs::read_dir(&puzzles_path)
        .await
        .with_context(|| format!("cannot list the puzzles in {puzzles_path:?}"))?;
    let mut indices = vec![];
    while let Some(directory_entry) = directory_entries
        .next_entry()
        .await
        .with_context(|| format!("cannot list the puzzles in {puzzles_path:?}"))?
    {
        let entry_path = directory_entry.path();
        // Each puzzle is a folder, possibly reached through a symlink, which `metadata` follows;
        // stray files such as `.DS_Store` are not puzzles.
        let metadata = tokio::fs::metadata(&entry_path)
            .await
            .with_context(|| format!("cannot tell whether {entry_path:?} is a folder"))?;
        if !metadata.is_dir() {
            continue;
        }

        // The name must be exactly the index's digits. `Puzzle::read` rebuilds the folder's name
        // from the index, so a folder named `007` would send it to a `7` folder that isn't there.
        let index = directory_entry
            .file_name()
            .to_str()
            .and_then(|name| {
                let index = name.parse::<usize>().ok()?;
                (index.to_string() == name).then_some(index)
            })
            .ok_or_else(|| {
                anyhow!(
                    "puzzle folder {entry_path:?} is not named by an index: expected digits \
                     without leading zeros, such as `10517`"
                )
            })?;
        indices.push(index);
    }
    indices.sort();
    Ok(indices)
}

// Reads every puzzle that has a stage-3 file, each paired with its index, in increasing order.
pub async fn read_clued_puzzles() -> anyhow::Result<Vec<(usize, Puzzle)>> {
    let mut clued_puzzles = vec![];
    for index in puzzle_indices().await? {
        let puzzle = Puzzle::read_if_present(index, CLUED_PUZZLE_FILE)
            .await
            .with_context(|| format!("cannot read puzzle {index}'s stage-3 file"))?;
        if let Some(puzzle) = puzzle {
            clued_puzzles.push((index, puzzle));
        }
    }
    Ok(clued_puzzles)
}
