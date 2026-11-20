// Tests for puzzle entries: renaming a clue's tag, and rejecting an entry that holds clue text
// instead of a tag. Renaming keeps puzzles in step when an edit changes a clue's tag in the clue
// bank.

use acrostic_generator::puzzle::{Clue, Puzzle};

// Returns a puzzle whose entries name the given `(answer, clue tag)` pairs.
fn make_puzzle(entries: &[(&str, &str)]) -> Puzzle {
    let clues = entries
        .iter()
        .map(|(answer, clue_tag)| Clue {
            clue_tag: Some(clue_tag.to_string()),
            answer: answer.to_string(),
            answer_letters: answer.to_uppercase(),
            indices: vec![],
        })
        .collect();
    Puzzle {
        quote: "Brevity is the soul of wit.".to_string(),
        quote_letters: None,
        source: "Shakespeare, Hamlet".to_string(),
        source_letters: None,
        clues: Some(clues),
        chat: None,
    }
}

// Returns the clue tag of each of the puzzle's entries, in order.
fn clue_tags(puzzle: &Puzzle) -> Vec<&str> {
    let clues = puzzle.clues.as_ref().expect("the puzzle should have clues");
    clues
        .iter()
        .map(|clue| clue.clue_tag.as_deref().unwrap_or(""))
        .collect()
}

#[test]
fn retagging_renames_the_tag_of_the_matching_entry() {
    let mut puzzle = make_puzzle(&[("nephew", "daughters-son"), ("redwood", "tall-tree")]);

    let is_changed = puzzle.retag_clue("nephew", "daughters-son", "siblings-son");

    assert!(is_changed);
    assert_eq!(vec!["siblings-son", "tall-tree"], clue_tags(&puzzle));
}

#[test]
fn retagging_skips_another_answer_that_shares_the_tag() {
    // Tags are unique only within one answer, so two answers can share a tag.
    let mut puzzle = make_puzzle(&[("nephew", "relative"), ("niece", "relative")]);

    puzzle.retag_clue("nephew", "relative", "siblings-son");

    assert_eq!(vec!["siblings-son", "relative"], clue_tags(&puzzle));
}

#[test]
fn retagging_a_tag_the_puzzle_lacks_changes_nothing() {
    let mut puzzle = make_puzzle(&[("nephew", "siblings-son")]);

    let is_changed = puzzle.retag_clue("nephew", "daughters-son", "relative");

    assert!(!is_changed);
    assert_eq!(vec!["siblings-son"], clue_tags(&puzzle));
}

#[test]
fn an_entry_holding_clue_text_fails_to_parse() {
    let json = r#"{
        "clue": "Sibling's son.",
        "answer": "nephew",
        "answer_letters": "NEPHEW",
        "indices": []
    }"#;

    let error = serde_json::from_str::<Clue>(json)
        .expect_err("an entry with a `clue` field should fail to parse");

    assert!(error.to_string().contains("unknown field"));
}
