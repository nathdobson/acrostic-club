// Tests for the clue bank: how a clue's tag comes from its text, how loading checks every tag
// against its clue, how adding and editing clues keep tags and reviews in step with text, and how
// reviews rank the clues new puzzles pick.

use std::path::Path;

use acrostic_generator::cluebank::{ClueBank, ClueEdit, ClueSource, Review, Reviewer, Verdict};

// Returns the tag the bank gives a clue with this text.
fn tag(text: &str) -> String {
    let mut bank = ClueBank::default();
    let clue = bank
        .add("answer", text, ClueSource::Person)
        .expect("the clue text should show something other than whitespace");
    clue.tag().to_string()
}

// Returns a bank holding one clue for each `(answer, text)` pair, each written by a person.
fn make_bank(clues: &[(&str, &str)]) -> ClueBank {
    let mut bank = ClueBank::default();
    for (answer, text) in clues {
        bank.add(answer, text, ClueSource::Person)
            .expect("each test clue should have a tag of its own");
    }
    bank
}

// Returns a review by a model with this verdict and no problems.
fn make_review(verdict: Verdict) -> Review {
    Review {
        reviewer: Reviewer::Model("claude-opus-5-5".to_string()),
        verdict,
        problems: vec![],
    }
}

// Returns a bank holding one clue for each `(answer, text, verdict)` triple, each written by a
// person and, unless `verdict` is `None`, reviewed by `make_review(verdict)`.
fn make_reviewed_bank(clues: &[(&str, &str, Option<Verdict>)]) -> ClueBank {
    let mut bank = ClueBank::default();
    for (answer, text, verdict) in clues {
        let tag = bank
            .add(answer, text, ClueSource::Person)
            .expect("each test clue should have a tag of its own")
            .tag()
            .to_string();
        if let Some(verdict) = verdict {
            bank.review(answer, &tag, make_review(*verdict))
                .expect("the clue just added should have this tag");
        }
    }
    bank
}

#[test]
fn a_tag_keeps_the_first_three_meaningful_words() {
    assert_eq!(
        "what-follows-battle",
        tag("What follows a battle or conflict.")
    );
}

#[test]
fn a_tag_drops_punctuation_inside_words() {
    assert_eq!("daughters-son", tag("Daughter's son."));
}

#[test]
fn a_tag_skips_a_blank_to_fill_in() {
    assert_eq!("people-spend-kroner", tag("___ people spend kroner."));
}

#[test]
fn a_tag_spells_accented_letters_in_ascii() {
    assert_eq!("pokemon-trainer", tag("Pokémon trainer"));
}

#[test]
fn a_tag_keeps_filler_words_when_the_clue_has_nothing_else() {
    assert_eq!("as-it-is", tag("As it is"));
}

#[test]
fn a_tag_names_the_symbols_of_a_clue_without_letters_or_digits() {
    assert_eq!("colon-hyphen-rightparen", tag(":-)"));
}

#[test]
fn a_tag_names_a_symbol_outside_ascii_by_its_ascii_form() {
    assert_eq!("hyphen-hyphen-hyphen", tag("— — —"));
}

#[test]
fn a_tag_names_a_symbol_without_an_ascii_form_by_its_code_point() {
    assert_eq!("u1d11e", tag("𝄞"));
}

#[test]
fn a_clue_of_only_whitespace_cannot_join_the_bank() {
    let mut bank = ClueBank::default();

    let error = bank
        .add("nephew", "  ", ClueSource::Person)
        .expect_err("a clue of only whitespace should have no tag");

    assert!(error.to_string().contains("whitespace"));
    assert!(bank.clues_for("nephew").is_empty());
}

#[test]
fn loading_accepts_a_bank_whose_tags_match_their_text() {
    let json = r#"{
        "nephew": [
            {
                "tag": "daughters-son",
                "text": "Daughter's son.",
                "source": { "model": "phi4" },
                "review": null
            }
        ]
    }"#;

    let bank = ClueBank::from_json(json).expect("a bank whose tags match should load");

    let clue = bank
        .find("nephew", "daughters-son")
        .expect("the bank should hold the clue");
    assert_eq!("Daughter's son.", clue.text());
}

#[test]
fn loading_reads_a_clues_review() {
    let json = r#"{
        "nephew": [
            {
                "tag": "siblings-son",
                "text": "Sibling's son.",
                "source": { "model": "phi4" },
                "review": {
                    "reviewer": { "model": "claude-opus-5-5" },
                    "verdict": "flat",
                    "problems": ["The clue is a dictionary definition."]
                }
            }
        ]
    }"#;

    let bank = ClueBank::from_json(json).expect("a bank with a review should load");

    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the bank should hold the clue");
    let expected_review = Review {
        reviewer: Reviewer::Model("claude-opus-5-5".to_string()),
        verdict: Verdict::Flat,
        problems: vec!["The clue is a dictionary definition.".to_string()],
    };
    assert_eq!(Some(&expected_review), clue.review());
}

#[test]
fn loading_rejects_a_tag_that_does_not_match_its_text() {
    let json = r#"{
        "nephew": [
            {
                "tag": "daughters-son",
                "text": "Sibling's son.",
                "source": "person",
                "review": null
            }
        ]
    }"#;

    let error = ClueBank::from_json(json).expect_err("a stale tag should fail to load");

    // The message names the tag the text calls for, so the fix is plain.
    assert!(error.to_string().contains("siblings-son"));
}

#[test]
fn loading_rejects_two_clues_with_one_tag_for_the_same_answer() {
    let json = r#"{
        "nephew": [
            {
                "tag": "siblings-son",
                "text": "Sibling's son.",
                "source": "person",
                "review": null
            },
            {
                "tag": "siblings-son",
                "text": "A sibling's son",
                "source": "person",
                "review": null
            }
        ]
    }"#;

    let error = ClueBank::from_json(json).expect_err("a repeated tag should fail to load");

    assert!(error.to_string().contains("more than one clue"));
}

#[test]
fn loading_rejects_an_answer_listed_twice() {
    let json = r#"{
        "nephew": [
            {
                "tag": "siblings-son",
                "text": "Sibling's son.",
                "source": "person",
                "review": null
            }
        ],
        "nephew": [
            {
                "tag": "brothers-son",
                "text": "Brother's son.",
                "source": "person",
                "review": null
            }
        ]
    }"#;

    let error = ClueBank::from_json(json).expect_err("a repeated answer should fail to load");

    assert!(format!("{error:#}").contains("more than once"));
}

#[test]
fn loading_rejects_a_clue_field_the_bank_does_not_know() {
    let json = r#"{
        "nephew": [
            {
                "tag": "siblings-son",
                "text": "Sibling's son.",
                "source": "person",
                "review": null,
                "note": "check with the editor"
            }
        ]
    }"#;

    let error = ClueBank::from_json(json).expect_err("an unknown field should fail to load");

    assert!(format!("{error:#}").contains("unknown field"));
}

#[test]
fn a_saved_bank_puts_each_field_on_its_own_line() {
    let mut bank = ClueBank::default();
    bank.add(
        "nephew",
        "Sibling's son.",
        ClueSource::Model("phi4".to_string()),
    )
    .expect("the clue should have a tag");

    let json = bank.to_json();

    let expected_json = concat!(
        "{\n",
        "  \"nephew\": [\n",
        "    {\n",
        "      \"tag\": \"siblings-son\",\n",
        "      \"text\": \"Sibling's son.\",\n",
        "      \"source\": {\n",
        "        \"model\": \"phi4\"\n",
        "      },\n",
        "      \"review\": null\n",
        "    }\n",
        "  ]\n",
        "}\n",
    );
    assert_eq!(expected_json, json);
}

#[test]
fn a_saved_bank_loads_back_unchanged() {
    let bank = make_reviewed_bank(&[
        ("nephew", "Sibling's son.", Some(Verdict::Flat)),
        (
            "nephew",
            "Family member who might call you aunt or uncle.",
            None,
        ),
        (
            "redwood",
            "It towers in California's forests.",
            Some(Verdict::Satisfying),
        ),
    ]);

    let reloaded_bank = ClueBank::from_json(&bank.to_json()).expect("a saved bank should load");

    assert_eq!(bank.clues_for("nephew"), reloaded_bank.clues_for("nephew"));
    assert_eq!(
        bank.clues_for("redwood"),
        reloaded_bank.clues_for("redwood")
    );
}

#[test]
fn adding_a_clue_tags_it_from_its_text() {
    let mut bank = ClueBank::default();

    let clue = bank
        .add(
            "aftermath",
            "What follows a battle or conflict.",
            ClueSource::Xd,
        )
        .expect("the clue should have a tag");

    assert_eq!("what-follows-battle", clue.tag());
}

#[test]
fn an_added_clue_starts_unreviewed() {
    let mut bank = ClueBank::default();

    let clue = bank
        .add(
            "aftermath",
            "What follows a battle or conflict.",
            ClueSource::Xd,
        )
        .expect("the clue should have a tag");

    assert_eq!(None, clue.review());
}

#[test]
fn adding_text_the_bank_already_holds_keeps_one_copy() {
    let mut bank = make_bank(&[("nephew", "Sibling's son.")]);

    bank.add("nephew", "Sibling's son.", ClueSource::Person)
        .expect("adding a clue twice should succeed");

    assert_eq!(1, bank.clues_for("nephew").len());
}

#[test]
fn adding_a_clue_whose_tag_names_a_different_clue_fails() {
    let mut bank = make_bank(&[("nephew", "Sibling's son.")]);

    let error = bank
        .add("nephew", "A sibling's son", ClueSource::Person)
        .expect_err("a clue whose tag is taken should not be added");

    assert!(error.to_string().contains("already names"));
    assert_eq!(1, bank.clues_for("nephew").len());
}

#[test]
fn puzzles_pick_among_an_answers_clues_by_their_indices() {
    let bank = make_bank(&[("nephew", "Sibling's son."), ("nephew", "Brother's son.")]);

    let clue_for_puzzle_10 = bank
        .clue_for_puzzle("nephew", 10)
        .expect("the answer should have clues");
    let clue_for_puzzle_11 = bank
        .clue_for_puzzle("nephew", 11)
        .expect("the answer should have clues");

    // Divided by the answer's two clues, the indices 10 and 11 leave the remainders 0 and 1.
    assert_eq!("Sibling's son.", clue_for_puzzle_10.text());
    assert_eq!("Brother's son.", clue_for_puzzle_11.text());
}

#[test]
fn puzzles_pick_only_among_an_answers_best_ranked_clues() {
    let bank = make_reviewed_bank(&[
        ("nephew", "Sister's son.", Some(Verdict::Flat)),
        ("nephew", "Sibling's son.", Some(Verdict::Satisfying)),
        ("nephew", "Brother's boy.", Some(Verdict::Satisfying)),
    ]);

    let clue_for_puzzle_10 = bank
        .clue_for_puzzle("nephew", 10)
        .expect("the answer should have clues");
    let clue_for_puzzle_11 = bank
        .clue_for_puzzle("nephew", 11)
        .expect("the answer should have clues");

    // Only the two satisfying clues are candidates, and divided by two, the indices 10 and 11
    // leave the remainders 0 and 1.
    assert_eq!("Sibling's son.", clue_for_puzzle_10.text());
    assert_eq!("Brother's boy.", clue_for_puzzle_11.text());
}

#[test]
fn a_flat_clue_outranks_an_unreviewed_one() {
    let bank = make_reviewed_bank(&[
        ("nephew", "Sibling's son.", None),
        ("nephew", "Sister's son.", Some(Verdict::Flat)),
    ]);

    let clue_for_puzzle_10 = bank
        .clue_for_puzzle("nephew", 10)
        .expect("the answer should have clues");
    let clue_for_puzzle_11 = bank
        .clue_for_puzzle("nephew", 11)
        .expect("the answer should have clues");

    // Were both clues candidates, the indices 10 and 11 would pick different ones.
    assert_eq!("Sister's son.", clue_for_puzzle_10.text());
    assert_eq!("Sister's son.", clue_for_puzzle_11.text());
}

#[test]
fn an_unreviewed_clue_outranks_a_flawed_one() {
    let bank = make_reviewed_bank(&[
        ("nephew", "Daughter's son.", Some(Verdict::Flawed)),
        ("nephew", "Sibling's son.", None),
    ]);

    let clue_for_puzzle_10 = bank
        .clue_for_puzzle("nephew", 10)
        .expect("the answer should have clues");
    let clue_for_puzzle_11 = bank
        .clue_for_puzzle("nephew", 11)
        .expect("the answer should have clues");

    // Were both clues candidates, the indices 10 and 11 would pick different ones.
    assert_eq!("Sibling's son.", clue_for_puzzle_10.text());
    assert_eq!("Sibling's son.", clue_for_puzzle_11.text());
}

#[test]
fn a_flawed_clue_still_serves_an_answer_that_has_no_other() {
    let bank = make_reviewed_bank(&[("nephew", "Daughter's son.", Some(Verdict::Flawed))]);

    let clue = bank
        .clue_for_puzzle("nephew", 10)
        .expect("the answer's only clue should serve, flawed or not");

    assert_eq!("Daughter's son.", clue.text());
}

#[test]
fn a_puzzle_finds_no_clue_for_an_answer_the_bank_lacks() {
    let bank = make_bank(&[("nephew", "Sibling's son.")]);
    assert!(bank.clue_for_puzzle("redwood", 10).is_none());
}

#[test]
fn editing_a_clue_retags_it_from_its_new_text() {
    let mut bank = make_bank(&[("nephew", "Daughter's son.")]);

    let new_tag = bank
        .edit(
            "nephew",
            "daughters-son",
            "Sibling's son.",
            ClueSource::Person,
        )
        .expect("the edit should succeed");

    assert_eq!("siblings-son", new_tag);
    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the clue should have its new tag");
    assert_eq!("Sibling's son.", clue.text());
    assert!(bank.find("nephew", "daughters-son").is_none());
}

#[test]
fn an_edit_credits_the_new_text_to_its_source() {
    let mut bank = ClueBank::default();
    bank.add("nephew", "Daughter's son.", ClueSource::Xd)
        .expect("the clue should have a tag");

    bank.edit(
        "nephew",
        "daughters-son",
        "Sibling's son.",
        ClueSource::Model("claude-fable-5-2".to_string()),
    )
    .expect("the edit should succeed");

    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the clue should have its new tag");
    let expected_source = ClueSource::Model("claude-fable-5-2".to_string());
    assert_eq!(&expected_source, clue.source());
}

#[test]
fn an_edit_that_changes_the_text_drops_the_review() {
    let mut bank = make_reviewed_bank(&[("nephew", "Daughter's son.", Some(Verdict::Flawed))]);

    bank.edit(
        "nephew",
        "daughters-son",
        "Sibling's son.",
        ClueSource::Person,
    )
    .expect("the edit should succeed");

    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the clue should have its new tag");
    assert_eq!(None, clue.review());
}

#[test]
fn an_edit_that_keeps_the_text_changes_nothing() {
    let mut bank = ClueBank::default();
    bank.add("nephew", "Sibling's son.", ClueSource::Xd)
        .expect("the clue should have a tag");
    bank.review("nephew", "siblings-son", make_review(Verdict::Flat))
        .expect("the clue should have this tag");

    bank.edit(
        "nephew",
        "siblings-son",
        "Sibling's son.",
        ClueSource::Person,
    )
    .expect("the edit should succeed");

    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the clue should keep its tag");
    assert_eq!(&ClueSource::Xd, clue.source());
    assert_eq!(
        Some(Verdict::Flat),
        clue.review().map(|review| review.verdict)
    );
}

#[test]
fn reviewing_a_clue_records_the_review() {
    let mut bank = make_bank(&[("nephew", "Sibling's son.")]);
    let review = Review {
        reviewer: Reviewer::Model("claude-opus-5-5".to_string()),
        verdict: Verdict::Flat,
        problems: vec!["The clue is a dictionary definition.".to_string()],
    };

    bank.review("nephew", "siblings-son", review)
        .expect("the review should succeed");

    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the clue should keep its tag");
    let expected_review = Review {
        reviewer: Reviewer::Model("claude-opus-5-5".to_string()),
        verdict: Verdict::Flat,
        problems: vec!["The clue is a dictionary definition.".to_string()],
    };
    assert_eq!(Some(&expected_review), clue.review());
}

#[test]
fn a_review_replaces_an_earlier_one() {
    let mut bank = make_reviewed_bank(&[("nephew", "Sibling's son.", Some(Verdict::Flat))]);

    bank.review("nephew", "siblings-son", make_review(Verdict::Satisfying))
        .expect("the review should succeed");

    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the clue should keep its tag");
    assert_eq!(
        Some(Verdict::Satisfying),
        clue.review().map(|review| review.verdict)
    );
}

#[test]
fn reviewing_a_clue_the_bank_lacks_fails() {
    let mut bank = make_bank(&[("nephew", "Sibling's son.")]);

    let error = bank
        .review("nephew", "daughters-son", make_review(Verdict::Flat))
        .expect_err("a review of a missing clue should fail");

    assert!(error.to_string().contains("no clue tagged"));
    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the existing clue should be untouched");
    assert_eq!(None, clue.review());
}

#[test]
fn a_persons_edit_is_credited_to_and_approved_by_a_person() {
    let edit = ClueEdit::by_person("nephew", "daughters-son", "Sibling's son.");

    let expected_edit = ClueEdit {
        answer: "nephew".to_string(),
        tag: "daughters-son".to_string(),
        new_text: "Sibling's son.".to_string(),
        source: ClueSource::Person,
        review: Some(Review {
            reviewer: Reviewer::Person,
            verdict: Verdict::Satisfying,
            problems: vec![],
        }),
    };
    assert_eq!(expected_edit, edit);
}

#[tokio::test]
async fn a_missing_bank_does_not_pass_for_a_missing_stage_file() {
    let error = ClueBank::read(Path::new("/nonexistent/cluebank.json"))
        .await
        .expect_err("a missing bank should fail to read");

    // `main` skips a puzzle whose stage file is missing by finding a not-found I/O error among an
    // error's causes, so a missing bank's error must carry no I/O error at all.
    assert!(error.downcast_ref::<std::io::Error>().is_none());
    assert!(error.to_string().contains("cannot read the clue bank"));
}

#[test]
fn editing_a_clue_onto_another_clues_tag_fails() {
    let mut bank = make_bank(&[("nephew", "Daughter's son."), ("nephew", "Sibling's son.")]);

    let error = bank
        .edit(
            "nephew",
            "daughters-son",
            "A sibling's son",
            ClueSource::Person,
        )
        .expect_err("an edit onto a taken tag should fail");

    assert!(error.to_string().contains("already names"));
    let clue = bank
        .find("nephew", "daughters-son")
        .expect("the clue should keep its tag");
    assert_eq!("Daughter's son.", clue.text());
}

#[test]
fn editing_a_clue_the_bank_lacks_fails() {
    let mut bank = make_bank(&[("nephew", "Sibling's son.")]);

    let error = bank
        .edit(
            "nephew",
            "daughters-son",
            "Brother's son.",
            ClueSource::Person,
        )
        .expect_err("an edit to a missing clue should fail");

    assert!(error.to_string().contains("no clue tagged"));
    let clue = bank
        .find("nephew", "siblings-son")
        .expect("the existing clue should be untouched");
    assert_eq!("Sibling's son.", clue.text());
}
