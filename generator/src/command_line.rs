// Parsing for the command line: the stage that `puzzle <target>` names, and the arguments that
// follow a target.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, anyhow};

// How to call `global edit-clue-as-person`, for the errors that report a malformed call.
const EDIT_CLUE_USAGE: &str = "usage: global edit-clue-as-person <answer> <tag> \"<new text>\"";

// How to call `global import-clues`, for the errors that report a malformed call.
const IMPORT_CLUES_USAGE: &str = "usage: global import-clues <path to a JSON array of clues>";

// The stages that `puzzle <target>` can run on each puzzle it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuzzleStage {
    Quote,
    Letters,
    Answers,
    Chat,
    RebuildCells,
}

// Each puzzle stage under the name that `puzzle <target>` calls it by.
const PUZZLE_STAGE_NAMES: &[(&str, PuzzleStage)] = &[
    ("quote", PuzzleStage::Quote),
    ("letters", PuzzleStage::Letters),
    ("answers", PuzzleStage::Answers),
    ("chat", PuzzleStage::Chat),
    ("rebuild-cells", PuzzleStage::RebuildCells),
];

// Parses the target of `puzzle <target>`, such as `answers`, as the stage it names.
pub fn parse_puzzle_stage(target: &str) -> anyhow::Result<PuzzleStage> {
    PUZZLE_STAGE_NAMES
        .iter()
        .find(|(name, _)| *name == target)
        .map(|&(_, stage)| stage)
        .with_context(|| {
            let names: Vec<&str> = PUZZLE_STAGE_NAMES.iter().map(|&(name, _)| name).collect();
            format!(
                "unknown puzzle target {target:?}; expected one of {}",
                names.join(", ")
            )
        })
}

// What the arguments after a target choose: the puzzles to work on, the language model to ask, and
// whether to save the work or only show it.
#[derive(Debug, Default, PartialEq)]
pub struct PuzzleArguments {
    // The puzzle numbers named one by one, such as `100`, or in inclusive ranges, such as
    // `100-200`.
    pub puzzle_numbers: BTreeSet<usize>,
    // The model that `--model <name>` names, if the arguments hold it.
    pub model: Option<String>,
    // Whether `--dry-run` asks to show the work without saving it.
    pub is_dry_run: bool,
}

// Parses puzzle numbers, ranges of them, `--model <name>`, and `--dry-run`, in any order.
pub fn parse_puzzle_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> anyhow::Result<PuzzleArguments> {
    let mut parsed = PuzzleArguments::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        // The flags hold hyphens, so they're checked before the hyphen that marks a range.
        if argument == "--model" {
            let model = arguments.next().context("`--model` needs a model name")?;
            parsed.model = Some(model);
        } else if argument == "--dry-run" {
            parsed.is_dry_run = true;
        } else if let Some((start, end)) = argument.split_once('-') {
            let start = parse_puzzle_number(start, &argument)?;
            let end = parse_puzzle_number(end, &argument)?;
            parsed.puzzle_numbers.extend(start..=end);
        } else {
            let puzzle_number = parse_puzzle_number(&argument, &argument)?;
            parsed.puzzle_numbers.insert(puzzle_number);
        }
    }
    Ok(parsed)
}

// What `global edit-clue-as-person` changes: one clue's text.
#[derive(Debug, PartialEq)]
pub struct EditClueArguments {
    pub answer: String,
    // The tag of the clue to change.
    pub tag: String,
    pub new_text: String,
}

// Parses the answer, the clue's tag, and the clue's new text, in that order.
pub fn parse_edit_clue_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> anyhow::Result<EditClueArguments> {
    let arguments: Vec<String> = arguments.into_iter().collect();
    // A fourth argument means the new text arrived unquoted, split into words.
    let [answer, tag, new_text] = <[String; 3]>::try_from(arguments).map_err(|arguments| {
        anyhow!(
            "expected an answer, a tag, and quoted text, but got {arguments:?}; {EDIT_CLUE_USAGE}"
        )
    })?;
    Ok(EditClueArguments {
        answer,
        tag,
        new_text,
    })
}

// Parses the one argument `global import-clues` takes: the path of the JSON file that lists the
// clues to import.
pub fn parse_import_clues_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> anyhow::Result<PathBuf> {
    let arguments: Vec<String> = arguments.into_iter().collect();
    let [path] = <[String; 1]>::try_from(arguments).map_err(|arguments| {
        anyhow!("expected one path, but got {arguments:?}; {IMPORT_CLUES_USAGE}")
    })?;
    Ok(PathBuf::from(path))
}

// Parses `text`, which is all or part of `argument`, as a puzzle number.
fn parse_puzzle_number(text: &str, argument: &str) -> anyhow::Result<usize> {
    text.parse().with_context(|| {
        format!("{argument:?} should be a puzzle number, or a range such as 100-200")
    })
}
