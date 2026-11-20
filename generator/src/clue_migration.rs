// A one-time step that moves clue text out of the puzzle files and into the clue bank.
//
// Stage-3 puzzle files hold each clue's text in a `clue` field. This step adds every such clue to
// the bank and replaces the field with `clue_tag`, which names the clue. Stage-2 files carry the
// same field, always empty, so they get an empty `clue_tag` instead.
//
// Every clue in the stage-3 files came from upstream commit 721df78, which asked phi4 for clues and
// fell back to the first clue the xd database listed for the answer. Which xd clue came first
// depended on the database's order at the time, and today's database may order its clues
// differently. So this step marks a clue as coming from xd whenever its text matches any xd clue
// for its answer word for word, and from phi4 otherwise. A phi4 clue that matches a published
// crossword clue word for word is thus credited to xd.

use anyhow::{Context, anyhow, bail, ensure};
use serde_json::Value;

use crate::cluebank::{ClueBank, ClueSource, clue_bank_path};
use crate::cluedb::{CLUE_DB, ClueDb};
use crate::puzzle::{CLUED_PUZZLE_FILE, Puzzle, puzzle_indices};
use crate::string::LetterString;
use crate::util::lazy_async::CloneError;
use crate::{PACKAGE_PATH, read_path_to_string};

// The stage file that holds a puzzle's chosen answers.
const ANSWERED_PUZZLE_FILE: &str = "stage2.json";

// The model that wrote every clue the xd database didn't supply.
const LEGACY_MODEL_NAME: &str = "phi4";

// Moves the clue text in every puzzle file into the clue bank, and writes the bank.
pub async fn migrate_clues() -> anyhow::Result<()> {
    let xd_clues = CLUE_DB.get().await.clone_error_static()?.clone();
    let mut bank = ClueBank::default();
    let mut moved_entry_count = 0;
    for index in puzzle_indices().await? {
        migrate_answered_puzzle(index).await?;
        moved_entry_count += migrate_clued_puzzle(index, &mut bank, &xd_clues).await?;
    }
    bank.write(&clue_bank_path()).await?;
    println!("moved {moved_entry_count} puzzle entries' clues into the clue bank");
    Ok(())
}

// Reads a puzzle's stage file as it was before this migration, with a `clue` field in each entry.
// Returns the puzzle together with the text each entry's `clue` field held, or `None` when the
// puzzle hasn't reached that stage.
async fn read_legacy_puzzle(
    index: usize,
    stage: &str,
) -> anyhow::Result<Option<(Puzzle, Vec<Option<String>>)>> {
    let path = PACKAGE_PATH
        .join("puzzles")
        .join(index.to_string())
        .join(stage);
    if !tokio::fs::try_exists(&path).await? {
        return Ok(None);
    }
    let json = read_path_to_string(&path).await?;
    let mut puzzle_value: Value =
        serde_json::from_str(&json).with_context(|| format!("cannot parse {path:?}"))?;

    // `Puzzle` rejects the `clue` field, so the field is taken out of each entry before parsing.
    let mut texts = vec![];
    if let Some(entries) = puzzle_value.get_mut("clues").and_then(Value::as_array_mut) {
        for entry in entries {
            let clue_value = entry
                .as_object_mut()
                .and_then(|fields| fields.remove("clue"));
            let text = match clue_value {
                None | Some(Value::Null) => None,
                Some(Value::String(text)) => Some(text),
                Some(other) => bail!("{path:?} holds a clue that isn't text: {other}"),
            };
            texts.push(text);
        }
    }
    let puzzle = serde_json::from_value(puzzle_value)
        .with_context(|| format!("cannot read {path:?} as a puzzle"))?;
    Ok(Some((puzzle, texts)))
}

// Rewrites a puzzle's stage-2 file with an empty `clue_tag` in place of its empty `clue` field.
async fn migrate_answered_puzzle(index: usize) -> anyhow::Result<()> {
    let Some((puzzle, texts)) = read_legacy_puzzle(index, ANSWERED_PUZZLE_FILE).await? else {
        return Ok(());
    };
    ensure!(
        texts.iter().all(Option::is_none),
        "puzzle {index}'s stage-2 file holds clue text, which only stage-3 files should"
    );
    puzzle.write(index, ANSWERED_PUZZLE_FILE).await?;
    Ok(())
}

// Adds the clues in a puzzle's stage-3 file to the bank and names each by its tag, returning how
// many entries moved.
async fn migrate_clued_puzzle(
    index: usize,
    bank: &mut ClueBank,
    xd_clues: &ClueDb,
) -> anyhow::Result<usize> {
    let Some((mut puzzle, texts)) = read_legacy_puzzle(index, CLUED_PUZZLE_FILE).await? else {
        return Ok(0);
    };
    let entries = puzzle
        .clues
        .as_mut()
        .ok_or_else(|| anyhow!("puzzle {index}'s stage-3 file has no entries"))?;
    for (entry, text) in entries.iter_mut().zip(texts) {
        let answer = &entry.answer;
        let text = text.ok_or_else(|| anyhow!("puzzle {index} has no clue text for {answer:?}"))?;
        let source = source_of(answer, &text, xd_clues);
        entry.clue_tag = Some(bank.add(answer, &text, source)?.tag().to_string());
    }
    let moved_entry_count = entries.len();
    puzzle.write(index, CLUED_PUZZLE_FILE).await?;
    Ok(moved_entry_count)
}

// Returns where an existing clue came from: xd when its text matches any clue the xd database
// lists for its answer, and phi4 otherwise.
fn source_of(answer: &str, text: &str, xd_clues: &ClueDb) -> ClueSource {
    let xd_entries = xd_clues.lookup(&LetterString::from_str(answer));
    if xd_entries.iter().any(|xd_entry| xd_entry.clue == text) {
        ClueSource::Xd
    } else {
        ClueSource::Model(LEGACY_MODEL_NAME.to_string())
    }
}
