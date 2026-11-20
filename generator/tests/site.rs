// Tests for the site build: `relative_path`, which lets build/site link to the checkout's files by
// paths that resolve inside whichever checkout holds the links, and publishing a puzzle, which
// replaces each clue's tag with the clue's text from the clue bank.

use std::io::ErrorKind;
use std::path::Path;

use acrostic_generator::cluebank::{ClueBank, ClueSource};
use acrostic_generator::puzzle::{Clue, Puzzle};
use acrostic_generator::site::{PublishedPuzzle, relative_path};
use serde_json::json;

#[test]
fn path_climbs_from_the_folder_to_the_shared_prefix() -> anyhow::Result<()> {
    let path = relative_path(
        Path::new("/checkout/player/index.html"),
        Path::new("/checkout/build/site"),
    )?;
    assert_eq!(Path::new("../../player/index.html"), path);
    Ok(())
}

// The generator's paths all start from `generator/..`, so a `..` inside the prefix the two paths
// share must not count as a folder to climb out of.
#[test]
fn dot_dot_inside_the_shared_prefix_is_not_climbed() -> anyhow::Result<()> {
    let path = relative_path(
        Path::new("/checkout/generator/../LICENSE"),
        Path::new("/checkout/generator/../build/site"),
    )?;
    assert_eq!(Path::new("../../LICENSE"), path);
    Ok(())
}

#[test]
fn dot_dot_past_the_shared_prefix_is_refused() {
    let error = relative_path(
        Path::new("/checkout/player/index.html"),
        Path::new("/checkout/build/../build/site"),
    )
    .expect_err("a `..` past the shared prefix should leave no way back to that prefix");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert!(error.to_string().contains("no relative path"), "{error}");
}

// Returns a bank holding one person-written clue for each `(answer, text)` pair.
fn make_bank(clues: &[(&str, &str)]) -> ClueBank {
    let mut bank = ClueBank::default();
    for (answer, text) in clues {
        bank.add(answer, text, ClueSource::Person)
            .expect("each test clue should have a tag of its own");
    }
    bank
}

// Returns a puzzle whose entries name the given `(answer, clue tag)` pairs.
fn make_puzzle(entries: &[(&str, Option<&str>)]) -> Puzzle {
    let clues = entries
        .iter()
        .map(|(answer, clue_tag)| Clue {
            clue_tag: clue_tag.map(str::to_string),
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

#[test]
fn publishing_replaces_each_tag_with_its_clue_text() {
    let bank = make_bank(&[("nephew", "Sibling's son."), ("redwood", "Tall tree")]);
    let puzzle = make_puzzle(&[
        ("nephew", Some("siblings-son")),
        ("redwood", Some("tall-tree")),
    ]);

    let published_puzzle = PublishedPuzzle::new(puzzle, &bank).expect("the puzzle should publish");

    let published_texts: Vec<&str> = published_puzzle
        .clues
        .iter()
        .map(|clue| clue.clue.as_str())
        .collect();
    assert_eq!(vec!["Sibling's son.", "Tall tree"], published_texts);
}

#[test]
fn a_published_puzzle_has_the_fields_the_player_reads() {
    let bank = make_bank(&[("nephew", "Sibling's son.")]);
    let puzzle = Puzzle {
        quote: "Brevity is the soul of wit.".to_string(),
        quote_letters: None,
        source: "Shakespeare, Hamlet".to_string(),
        source_letters: None,
        clues: Some(vec![Clue {
            clue_tag: Some("siblings-son".to_string()),
            answer: "nephew".to_string(),
            answer_letters: "NEPHEW".to_string(),
            indices: vec![4, 0, 2],
        }]),
        chat: None,
    };

    let published_puzzle = PublishedPuzzle::new(puzzle, &bank).expect("the puzzle should publish");

    let expected_json = json!({
        "quote": "Brevity is the soul of wit.",
        "quote_letters": null,
        "source": "Shakespeare, Hamlet",
        "source_letters": null,
        "clues": [
            {
                "clue": "Sibling's son.",
                "answer": "nephew",
                "answer_letters": "NEPHEW",
                "indices": [4, 0, 2],
            },
        ],
        "chat": null,
    });
    let published_json =
        serde_json::to_value(&published_puzzle).expect("a published puzzle should serialize");
    assert_eq!(expected_json, published_json);
}

#[test]
fn publishing_fails_when_a_tag_names_a_clue_the_bank_lacks() {
    let bank = make_bank(&[("nephew", "Sibling's son.")]);
    let puzzle = make_puzzle(&[("nephew", Some("daughters-son"))]);

    let error =
        PublishedPuzzle::new(puzzle, &bank).expect_err("a missing clue should fail to publish");

    assert!(error.to_string().contains("daughters-son"));
}

#[test]
fn publishing_fails_when_an_entry_names_no_clue() {
    let bank = make_bank(&[("nephew", "Sibling's son.")]);
    let puzzle = make_puzzle(&[("nephew", None)]);

    let error =
        PublishedPuzzle::new(puzzle, &bank).expect_err("an entry without a clue should fail");

    assert!(error.to_string().contains("names no clue"));
}
