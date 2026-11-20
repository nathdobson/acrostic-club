// Parsing for the command-line arguments that follow a target.

use std::collections::BTreeSet;

use anyhow::{Context, anyhow};

use crate::cluebank::Reviewer;

// How to call `global edit-clue`, for the errors that report a malformed call.
const EDIT_CLUE_USAGE: &str =
    "usage: global edit-clue --reviewer <model name | person> <answer> <tag> \"<new text>\"";

// What the arguments after a target choose: the puzzles to work on, and the language model to ask.
#[derive(Debug, Default, PartialEq)]
pub struct PuzzleArguments {
    // The puzzle numbers named one by one, such as `100`, or in inclusive ranges, such as
    // `100-200`.
    pub puzzle_numbers: BTreeSet<usize>,
    // The model that `--model <name>` names, if the arguments hold it.
    pub model: Option<String>,
}

// Parses puzzle numbers, ranges of them, and `--model <name>`, in any order.
pub fn parse_puzzle_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> anyhow::Result<PuzzleArguments> {
    let mut parsed = PuzzleArguments::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        // The flag holds a hyphen, so it's checked before the hyphen that marks a range.
        if argument == "--model" {
            let model = arguments.next().context("`--model` needs a model name")?;
            parsed.model = Some(model);
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

// What `global edit-clue` changes: one clue's text, and who reviewed the change.
#[derive(Debug, PartialEq)]
pub struct EditClueArguments {
    pub answer: String,
    // The tag of the clue to change.
    pub tag: String,
    pub new_text: String,
    pub reviewer: Reviewer,
}

// Parses the answer, the clue's tag, and the clue's new text, in that order, plus the `--reviewer`
// flag, which may come anywhere among them. The flag names a model, or `person` for a person. It
// is required, so that an edit a model makes is never credited to a person by default.
pub fn parse_edit_clue_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> anyhow::Result<EditClueArguments> {
    let mut reviewer = None;
    let mut positional_arguments = vec![];
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == "--reviewer" {
            let name = arguments.next().with_context(|| {
                format!("`--reviewer` needs a model name or `person`; {EDIT_CLUE_USAGE}")
            })?;
            reviewer = Some(if name == "person" {
                Reviewer::Person
            } else {
                Reviewer::Model(name)
            });
        } else {
            positional_arguments.push(argument);
        }
    }
    let reviewer =
        reviewer.with_context(|| format!("`--reviewer` is required; {EDIT_CLUE_USAGE}"))?;

    // A fourth argument means the new text arrived unquoted, split into words.
    let [answer, tag, new_text] =
        <[String; 3]>::try_from(positional_arguments).map_err(|positional_arguments| {
            anyhow!(
                "expected an answer, a tag, and quoted text, but got {positional_arguments:?}; \
                 {EDIT_CLUE_USAGE}"
            )
        })?;
    Ok(EditClueArguments {
        answer,
        tag,
        new_text,
        reviewer,
    })
}

// Parses `text`, which is all or part of `argument`, as a puzzle number.
fn parse_puzzle_number(text: &str, argument: &str) -> anyhow::Result<usize> {
    text.parse().with_context(|| {
        format!("{argument:?} should be a puzzle number, or a range such as 100-200")
    })
}
