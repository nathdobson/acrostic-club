// Parsing for the command-line arguments that follow a target.

use std::collections::BTreeSet;

use anyhow::Context;

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

// Parses `text`, which is all or part of `argument`, as a puzzle number.
fn parse_puzzle_number(text: &str, argument: &str) -> anyhow::Result<usize> {
    text.parse().with_context(|| {
        format!("{argument:?} should be a puzzle number, or a range such as 100-200")
    })
}
