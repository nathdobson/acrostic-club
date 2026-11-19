// Tests for the parsers of the arguments that follow a target on the command line:
// `parse_puzzle_arguments`, which reads the puzzle numbers, model, and dry-run flag, and
// `parse_edit_clue_arguments`, which reads what `global edit-clue` changes.

use std::collections::BTreeSet;

use acrostic_generator::cluebank::Reviewer;
use acrostic_generator::command_line::{
    EditClueArguments, PuzzleArguments, parse_edit_clue_arguments, parse_puzzle_arguments,
};

fn make_arguments(arguments: &[&str]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| argument.to_string())
        .collect()
}

#[test]
fn puzzle_numbers_and_ranges_name_every_puzzle_they_cover() -> anyhow::Result<()> {
    let parsed = parse_puzzle_arguments(make_arguments(&["5", "10-12"]))?;

    assert_eq!(
        parsed,
        PuzzleArguments {
            puzzle_numbers: BTreeSet::from([5, 10, 11, 12]),
            model: None,
            is_dry_run: false,
        }
    );
    Ok(())
}

#[test]
fn a_model_may_come_among_the_puzzle_numbers() -> anyhow::Result<()> {
    let parsed = parse_puzzle_arguments(make_arguments(&["5", "--model", "gemma4:12b-qat", "6"]))?;

    assert_eq!(
        parsed,
        PuzzleArguments {
            puzzle_numbers: BTreeSet::from([5, 6]),
            model: Some("gemma4:12b-qat".to_string()),
            is_dry_run: false,
        }
    );
    Ok(())
}

#[test]
fn a_dry_run_flag_may_come_among_the_puzzle_numbers() -> anyhow::Result<()> {
    let parsed = parse_puzzle_arguments(make_arguments(&["5", "--dry-run", "6"]))?;

    assert_eq!(
        parsed,
        PuzzleArguments {
            puzzle_numbers: BTreeSet::from([5, 6]),
            model: None,
            is_dry_run: true,
        }
    );
    Ok(())
}

#[test]
fn a_model_flag_without_a_name_fails() {
    let error = parse_puzzle_arguments(make_arguments(&["5", "--model"]))
        .expect_err("the flag names no model");

    assert!(error.to_string().contains("--model"));
}

#[test]
fn an_argument_that_isnt_a_puzzle_number_fails() {
    let error = parse_puzzle_arguments(make_arguments(&["5", "ten"]))
        .expect_err("`ten` isn't a puzzle number");

    assert!(error.to_string().contains("ten"));
}

#[test]
fn a_range_with_a_bad_end_fails() {
    let error =
        parse_puzzle_arguments(make_arguments(&["10-x"])).expect_err("`x` isn't a puzzle number");

    assert!(error.to_string().contains("10-x"));
}

#[test]
fn an_edit_takes_its_reviewer_from_anywhere_among_its_arguments() -> anyhow::Result<()> {
    let parsed = parse_edit_clue_arguments(make_arguments(&[
        "nephew",
        "daughters-son",
        "--reviewer",
        "claude-opus-5-5",
        "Sibling's son.",
    ]))?;

    assert_eq!(
        parsed,
        EditClueArguments {
            answer: "nephew".to_string(),
            tag: "daughters-son".to_string(),
            new_text: "Sibling's son.".to_string(),
            reviewer: Reviewer::Model("claude-opus-5-5".to_string()),
        }
    );
    Ok(())
}

#[test]
fn an_edit_reviewed_by_person_is_credited_to_a_person() -> anyhow::Result<()> {
    let parsed = parse_edit_clue_arguments(make_arguments(&[
        "--reviewer",
        "person",
        "nephew",
        "daughters-son",
        "Sibling's son.",
    ]))?;

    assert_eq!(parsed.reviewer, Reviewer::Person);
    Ok(())
}

#[test]
fn an_edit_without_a_reviewer_fails() {
    let error = parse_edit_clue_arguments(make_arguments(&[
        "nephew",
        "daughters-son",
        "Sibling's son.",
    ]))
    .expect_err("an edit should need a reviewer");

    assert!(error.to_string().contains("--reviewer"));
}

#[test]
fn an_edit_whose_text_arrives_unquoted_fails() {
    let error = parse_edit_clue_arguments(make_arguments(&[
        "--reviewer",
        "person",
        "nephew",
        "daughters-son",
        "Sibling's",
        "son.",
    ]))
    .expect_err("text split into words should fail");

    assert!(error.to_string().contains("quoted text"));
}
