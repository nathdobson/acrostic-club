// Tests for `rebuild_cells`, which brings a committed puzzle's cells up to date with the current
// `quote_to_cells`.

use acrostic_generator::maintenance::rebuild_cells::rebuild_cells;
use acrostic_generator::puzzle::{Clue, Puzzle};

// Builds a puzzle whose committed cells may predate the current `quote_to_cells`.
fn make_puzzle(quote: &str, cells: &str, clues: Option<Vec<Clue>>) -> Puzzle {
    Puzzle {
        quote: quote.to_string(),
        quote_letters: Some(cells.to_string()),
        source: String::new(),
        source_letters: None,
        clues,
        chat: None,
    }
}

// Builds a clue whose answer's letters sit at the given cell indices.
fn make_clue(answer: &str, indices: Vec<usize>) -> Clue {
    Clue {
        clue: None,
        answer: answer.to_string(),
        answer_letters: answer.to_string(),
        indices,
    }
}

#[test]
fn a_rebuild_shifts_the_clue_indices_after_a_new_word_break() {
    let clues = vec![make_clue("BED", vec![3, 0, 2])];
    let mut puzzle = make_puzzle("end.Begin", "ENDBEGIN", Some(clues));

    rebuild_cells(&mut puzzle).expect("only a space should change");

    assert_eq!(puzzle.quote_letters.as_deref(), Some("END BEGIN"));
    let clues = puzzle.clues.expect("the clues should remain");
    assert_eq!(clues[0].indices, vec![4, 0, 2]);
}

#[test]
fn a_rebuild_shifts_the_clue_indices_after_a_new_digit() {
    // The committed cells lack the digit, as if an older `quote_to_cells` had dropped it.
    let clues = vec![make_clue("BED", vec![3, 0, 2])];
    let mut puzzle = make_puzzle("end3Begin", "ENDBEGIN", Some(clues));

    rebuild_cells(&mut puzzle).expect("only a digit should change");

    assert_eq!(puzzle.quote_letters.as_deref(), Some("END3BEGIN"));
    let clues = puzzle.clues.expect("the clues should remain");
    assert_eq!(clues[0].indices, vec![4, 0, 2]);
}

#[test]
fn a_rebuild_shifts_the_clue_indices_after_a_new_hyphen() {
    // The committed cells lack the hyphen, as if an older `quote_to_cells` had dropped it.
    let clues = vec![make_clue("OWL", vec![6, 0, 2])];
    let mut puzzle = make_puzzle("well-known", "WELLKNOWN", Some(clues));

    rebuild_cells(&mut puzzle).expect("only a hyphen should change");

    assert_eq!(puzzle.quote_letters.as_deref(), Some("WELL-KNOWN"));
    let clues = puzzle.clues.expect("the clues should remain");
    assert_eq!(clues[0].indices, vec![7, 0, 2]);
}

#[test]
fn a_rebuild_updates_the_cells_of_a_puzzle_without_clues() {
    let mut puzzle = make_puzzle("end.Begin", "ENDBEGIN", None);

    rebuild_cells(&mut puzzle).expect("only a space should change");

    assert_eq!(puzzle.quote_letters.as_deref(), Some("END BEGIN"));
}

#[test]
fn a_rebuild_rejects_a_changed_letter_and_changes_nothing() {
    // The committed cells spell BEGAN, but the quote spells Begin.
    let clues = vec![make_clue("BED", vec![3, 0, 2])];
    let mut puzzle = make_puzzle("end.Begin", "ENDBEGAN", Some(clues));

    let error = rebuild_cells(&mut puzzle).expect_err("a changed letter should fail the rebuild");

    assert!(error.to_string().contains("letters changed"));
    assert_eq!(puzzle.quote_letters.as_deref(), Some("ENDBEGAN"));
    let clues = puzzle.clues.expect("the clues should remain");
    assert_eq!(clues[0].indices, vec![3, 0, 2]);
}

#[test]
fn a_rebuild_rejects_a_clue_on_a_non_letter_cell_and_changes_nothing() {
    // The second clue points at the digit 3, a cell no answer can spell.
    let clues = vec![make_clue("BED", vec![3, 0, 2]), make_clue("N", vec![8])];
    let mut puzzle = make_puzzle("end.Begin3", "ENDBEGIN3", Some(clues));

    let error = rebuild_cells(&mut puzzle).expect_err("a non-letter cell should fail the rebuild");

    assert!(error.to_string().contains("non-letter cell"));
    assert_eq!(puzzle.quote_letters.as_deref(), Some("ENDBEGIN3"));
    let clues = puzzle.clues.expect("the clues should remain");
    assert_eq!(clues[0].indices, vec![3, 0, 2]);
}
