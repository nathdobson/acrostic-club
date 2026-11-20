// Parsing for the command-line arguments that follow a target.

use std::collections::BTreeSet;

use anyhow::{Context, anyhow};

// How to call `global edit-clue-as-person`, for the errors that report a malformed call.
const EDIT_CLUE_USAGE: &str = "usage: global edit-clue-as-person <answer> <tag> \"<new text>\"";

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

// Parses `text`, which is all or part of `argument`, as a puzzle number.
fn parse_puzzle_number(text: &str, argument: &str) -> anyhow::Result<usize> {
    text.parse().with_context(|| {
        format!("{argument:?} should be a puzzle number, or a range such as 100-200")
    })
}
