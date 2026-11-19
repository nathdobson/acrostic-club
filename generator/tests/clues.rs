// Tests for `ClueClient` and the dry run of the `chat` step. Most check what the dry run shows for
// each answer: the clue bank's clues above the clue just written for it. One test, ignored by
// default, prints the clue a running language model writes for a word, from the model
// `ACROSTIC_TEST_MODEL` names or else the default, as in
// `ACROSTIC_TEST_MODEL=gemma4:26b-qat cargo test --test clues -- --ignored`.

use std::collections::HashMap;
use std::{env, mem};

use acrostic_generator::cluebank::{ClueBank, ClueSource};
use acrostic_generator::clues::{ClueClient, ClueWriter, preview_clues};
use acrostic_generator::llm::DEFAULT_MODEL_NAME;
use acrostic_generator::puzzle::{Clue, Puzzle};
use acrostic_generator::util::interrupt::channel;

// A clue writer that returns a scripted clue for each answer it knows, and no clue for any other.
struct ScriptedClueWriter {
    clues_by_answer: HashMap<String, (String, ClueSource)>,
}

impl ClueWriter for ScriptedClueWriter {
    async fn write_clue(&self, answer: &str) -> anyhow::Result<Option<(String, ClueSource)>> {
        Ok(self.clues_by_answer.get(answer).cloned())
    }
}

// Returns a writer that answers each `(answer, text, model)` with clue `text`, written by `model`.
fn make_writer(clues: &[(&str, &str, &str)]) -> ScriptedClueWriter {
    let clues_by_answer = clues
        .iter()
        .map(|(answer, text, model)| {
            let source = ClueSource::Model(model.to_string());
            (answer.to_string(), (text.to_string(), source))
        })
        .collect();
    ScriptedClueWriter { clues_by_answer }
}

// Returns a puzzle whose entries hold `answers`, in order, with every field the preview doesn't
// read left empty.
fn make_puzzle(answers: &[&str]) -> Puzzle {
    let entries = answers
        .iter()
        .map(|answer| Clue {
            clue_tag: None,
            answer: answer.to_string(),
            answer_letters: String::new(),
            indices: vec![],
        })
        .collect();
    Puzzle {
        quote: String::new(),
        quote_letters: None,
        source: String::new(),
        source_letters: None,
        clues: Some(entries),
        chat: None,
    }
}

#[tokio::test]
async fn a_preview_writes_a_new_clue_even_for_an_answer_the_bank_covers() -> anyhow::Result<()> {
    let mut bank = ClueBank::default();
    bank.add("argent", "Silver, in heraldry", ClueSource::Xd)?;
    bank.add("argent", "Metal on a coat of arms", ClueSource::Person)?;
    let writer = make_writer(&[("argent", "Heraldic silver", "gemma4:31b-qat")]);

    let preview = preview_clues(&make_puzzle(&["argent"]), &bank, &writer).await?;

    assert_eq!(
        concat!(
            "argent\n",
            "    bank: Silver, in heraldry  [xd]\n",
            "    bank: Metal on a coat of arms  [person]\n",
            "    new:  Heraldic silver  [gemma4:31b-qat]\n",
        ),
        preview
    );
    Ok(())
}

#[tokio::test]
async fn a_preview_shows_none_when_the_bank_has_no_clue() -> anyhow::Result<()> {
    let bank = ClueBank::default();
    let writer = make_writer(&[("argent", "Heraldic silver", "phi4")]);

    let preview = preview_clues(&make_puzzle(&["argent"]), &bank, &writer).await?;

    assert_eq!(
        concat!(
            "argent\n",
            "    bank: (none)\n",
            "    new:  Heraldic silver  [phi4]\n",
        ),
        preview
    );
    Ok(())
}

#[tokio::test]
async fn a_preview_shows_none_when_no_new_clue_was_written() -> anyhow::Result<()> {
    let mut bank = ClueBank::default();
    bank.add("argent", "Silver, in heraldry", ClueSource::Xd)?;
    let writer = make_writer(&[]);

    let preview = preview_clues(&make_puzzle(&["argent"]), &bank, &writer).await?;

    assert_eq!(
        concat!(
            "argent\n",
            "    bank: Silver, in heraldry  [xd]\n",
            "    new:  (none)\n",
        ),
        preview
    );
    Ok(())
}

#[tokio::test]
async fn a_preview_shows_answers_in_the_order_given() -> anyhow::Result<()> {
    let bank = ClueBank::default();
    let writer = make_writer(&[
        ("nephew", "Son of a sibling", "gemma4:12b-qat"),
        ("dublin", "Irish capital city", "gemma4:12b-qat"),
    ]);

    let preview = preview_clues(&make_puzzle(&["nephew", "dublin"]), &bank, &writer).await?;

    assert_eq!(
        concat!(
            "nephew\n",
            "    bank: (none)\n",
            "    new:  Son of a sibling  [gemma4:12b-qat]\n",
            "dublin\n",
            "    bank: (none)\n",
            "    new:  Irish capital city  [gemma4:12b-qat]\n",
        ),
        preview
    );
    Ok(())
}

#[tokio::test]
#[ignore = "needs a running local language model"]
async fn a_clue_client_writes_a_clue() -> anyhow::Result<()> {
    let model = env::var("ACROSTIC_TEST_MODEL").unwrap_or(DEFAULT_MODEL_NAME.to_string());
    let (cleanup_sender, cleanup_receiver) = channel();
    let client = ClueClient::new(cleanup_sender, &model).await?;

    for word in &[
        // "extant", "netball",
        // "nathan",
        // "andrew", "john", "hindwings"
        // "dudley",
        // "atkinson",
        // "howell",
        "india's",
    ] {
        let clue = client.create_clue(word).await?;
        println!("{clue:?}");
    }

    mem::drop(client);
    cleanup_receiver.cleanup().await?;
    Ok(())
}
