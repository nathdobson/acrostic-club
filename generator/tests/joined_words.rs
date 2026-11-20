// Tests for the joined-word bank: loading it, adding the splits a language model proposes, and
// applying it to quotes.

use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::sync::Mutex;

use acrostic_generator::joined_words::{JoinedWordBank, ProposalRejection, add_model_proposals};
use acrostic_generator::llm::chat_client::{BaseClient, ChatClient};
use acrostic_generator::llm::rpcs::JoinedWord;
use futures::FutureExt;
use futures::future::BoxFuture;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse};

fn make_joined_word(joined: &str, split: &str) -> JoinedWord {
    JoinedWord {
        joined: joined.to_string(),
        split: split.to_string(),
    }
}

// Builds a bank holding each joined word with its split, and the given words to keep whole.
fn make_bank(splits: &[(&str, &str)], keep_whole: &[&str]) -> JoinedWordBank {
    JoinedWordBank {
        keep_whole: keep_whole.iter().map(|word| word.to_string()).collect(),
        splits: splits
            .iter()
            .map(|&(joined, split)| (joined.to_string(), split.to_string()))
            .collect(),
    }
}

#[test]
fn a_bank_reads_back_what_it_writes() -> anyhow::Result<()> {
    // The apostrophes need the keys quoted.
    let bank = make_bank(
        &[
            ("theyhad", "they had"),
            ("That’swhat", "That’s what"),
            ("I'dreached", "I'd reached"),
        ],
        &["doorjamb", "churchgoin"],
    );

    let read_back = JoinedWordBank::from_toml(&bank.to_toml(&[]))?;

    assert_eq!(read_back, bank);
    Ok(())
}

#[test]
fn a_written_bank_opens_with_a_description_of_its_fields() {
    let toml = make_bank(&[("theyhad", "they had")], &["doorjamb"]).to_toml(&[]);

    assert!(toml.starts_with("# The joined-word bank"), "{toml}");
    assert!(toml.contains("# keep_whole: "), "{toml}");
    assert!(toml.contains("# splits: "), "{toml}");
}

#[test]
fn a_written_bank_lists_the_puzzles_holding_each_word() {
    let bank = make_bank(&[("theyhad", "they had")], &["doorjamb"]);
    let publishable_quotes = [(7, "theyhad left"), (9, "the doorjamb"), (12, "so theyhad")];

    let toml = bank.to_toml(&publishable_quotes);

    assert!(
        toml.contains("\ntheyhad = \"they had\"  # 7, 12\n"),
        "{toml}"
    );
    assert!(toml.contains("\n    \"doorjamb\",  # 9\n"), "{toml}");
}

#[test]
fn a_written_bank_marks_a_word_that_no_publishable_puzzle_holds() {
    let bank = make_bank(&[("theyhad", "they had")], &[]);

    let toml = bank.to_toml(&[(7, "they had left")]);

    assert!(
        toml.contains("\ntheyhad = \"they had\"  # no publishable puzzle\n"),
        "{toml}"
    );
}

#[test]
fn a_split_that_changes_the_word_fails_to_load() {
    let toml = make_bank(&[("wereredecorating", "were decorating")], &[]).to_toml(&[]);

    let error = JoinedWordBank::from_toml(&toml).expect_err("the split drops two letters");

    assert!(error.to_string().contains("wereredecorating"));
}

#[test]
fn a_split_with_a_doubled_space_fails_to_load() {
    let toml = make_bank(&[("theyhad", "they  had")], &[]).to_toml(&[]);

    let error = JoinedWordBank::from_toml(&toml).expect_err("the split doubles its space");

    assert!(error.to_string().contains("theyhad"));
}

#[test]
fn a_split_with_a_space_beside_punctuation_fails_to_load() {
    let toml = make_bank(&[("her.He", "her. He")], &[]).to_toml(&[]);

    let error = JoinedWordBank::from_toml(&toml).expect_err("the space follows a period");

    assert!(error.to_string().contains("her.He"));
}

#[test]
fn a_word_both_split_and_kept_whole_fails_to_load() {
    let toml = make_bank(&[("ubervillains", "uber villains")], &["ubervillains"]).to_toml(&[]);

    let error = JoinedWordBank::from_toml(&toml).expect_err("the word is in both lists");

    assert!(error.to_string().contains("ubervillains"));
}

#[test]
fn a_word_kept_whole_with_punctuation_at_an_end_fails_to_load() {
    let toml = make_bank(&[], &["churchgoin'"]).to_toml(&[]);

    let error = JoinedWordBank::from_toml(&toml).expect_err("the word ends in an apostrophe");

    assert!(error.to_string().contains("churchgoin'"));
}

#[test]
fn a_proposal_adds_its_split_to_the_bank() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("wereredecorating", "were redecorating");

    let result = bank.add_proposal("they wereredecorating it", &proposal);

    assert_eq!(result, Ok(()));
    let expected = BTreeMap::from([(
        "wereredecorating".to_string(),
        "were redecorating".to_string(),
    )]);
    assert_eq!(bank.splits, expected);
}

#[test]
fn a_proposal_whose_split_changes_a_letters_case_is_rejected() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("solitudeIs", "solitude is");

    let result = bank.add_proposal("that solitudeIs not", &proposal);

    assert_eq!(result, Err(ProposalRejection::MisspellsTheWord));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_that_straightens_the_quotes_apostrophe_is_rejected() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("don'task", "don't ask");

    let result = bank.add_proposal("you don’task me", &proposal);

    assert_eq!(result, Err(ProposalRejection::NotInQuote));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_drops_punctuation_around_the_word() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("plungeSoul-forward,", "plunge Soul-forward,");

    let result = bank.add_proposal("I plungeSoul-forward, onward", &proposal);

    assert_eq!(result, Ok(()));
    assert_eq!(bank.splits["plungeSoul-forward"], "plunge Soul-forward");
}

#[test]
fn a_proposal_drops_a_space_beside_punctuation() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("geta…I", "get a… I");

    let result = bank.add_proposal("to geta…I said", &proposal);

    assert_eq!(result, Ok(()));
    assert_eq!(bank.splits["geta…I"], "get a…I");
}

#[test]
fn a_proposal_with_spaces_only_beside_punctuation_is_rejected() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("her.He", "her. He");

    let result = bank.add_proposal("for her.He left", &proposal);

    assert_eq!(result, Err(ProposalRejection::SplitsNothing));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_of_a_split_the_bank_holds_is_accepted() {
    let mut bank = make_bank(&[("theyhad", "they had")], &[]);
    let proposal = make_joined_word("theyhad", "they had");

    let result = bank.add_proposal("theyhad left", &proposal);

    assert_eq!(result, Ok(()));
    assert_eq!(bank, make_bank(&[("theyhad", "they had")], &[]));
}

#[test]
fn a_proposal_that_misspells_the_word_is_rejected() {
    let mut bank = JoinedWordBank::default();
    // Splitting "wereredecorating" into "were decorating" would drop two letters.
    let proposal = make_joined_word("wereredecorating", "were decorating");

    let result = bank.add_proposal("they wereredecorating it", &proposal);

    assert_eq!(result, Err(ProposalRejection::MisspellsTheWord));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_into_one_word_is_rejected() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("cannot", "cannot");

    let result = bank.add_proposal("I cannot", &proposal);

    assert_eq!(result, Err(ProposalRejection::SplitsNothing));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_for_part_of_a_longer_word_is_rejected() {
    let mut bank = JoinedWordBank::default();
    let proposal = make_joined_word("andspend", "and spend");

    let result = bank.add_proposal("andspending", &proposal);

    assert_eq!(result, Err(ProposalRejection::NotInQuote));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_to_split_a_word_kept_whole_is_rejected() {
    let mut bank = make_bank(&[], &["ubervillains"]);
    let proposal = make_joined_word("Ubervillains", "Uber villains");

    let result = bank.add_proposal("Ubervillains, all", &proposal);

    assert_eq!(result, Err(ProposalRejection::MustRemainJoined));
    assert!(bank.splits.is_empty());
}

#[test]
fn a_proposal_that_conflicts_with_the_bank_is_rejected() {
    let mut bank = make_bank(&[("moonincautious", "moon incautious")], &[]);
    let proposal = make_joined_word("moonincautious", "moon in cautious");

    let result = bank.add_proposal("Autumn moonincautious in the dark", &proposal);

    assert_eq!(result, Err(ProposalRejection::ConflictsWithBank));
    assert_eq!(bank.splits["moonincautious"], "moon incautious");
}

#[test]
fn the_bank_splits_a_joined_word_in_a_quote() {
    let bank = make_bank(&[("alibrary", "a library")], &[]);

    let quote = bank.split_joined_words("build alibrary worthy of your soul");

    assert_eq!(quote, "build a library worthy of your soul");
}

#[test]
fn the_bank_splits_every_occurrence_of_a_joined_word() {
    let bank = make_bank(&[("andso", "and so")], &[]);

    let quote = bank.split_joined_words("andso it went, andso");

    assert_eq!(quote, "and so it went, and so");
}

#[test]
fn the_bank_leaves_a_longer_word_alone() {
    let bank = make_bank(&[("andspend", "and spend")], &[]);

    let quote = bank.split_joined_words("andspending");

    assert_eq!(quote, "andspending");
}

// A client that answers each request with the next of its scripted replies, and records the
// requests it gets.
struct ScriptedClient {
    replies: Mutex<VecDeque<String>>,
    requests: Mutex<Vec<ChatMessageRequest>>,
}

impl ScriptedClient {
    fn new(replies: &[&str]) -> Self {
        ScriptedClient {
            replies: Mutex::new(replies.iter().map(|reply| reply.to_string()).collect()),
            requests: Mutex::new(vec![]),
        }
    }
}

impl ChatClient for ScriptedClient {
    fn send_chat_messages<'a>(
        &'a self,
        input: &'a ChatMessageRequest,
    ) -> BoxFuture<'a, anyhow::Result<ChatMessageResponse>> {
        self.requests
            .lock()
            .expect("no test should panic while holding the request lock")
            .push(input.clone());
        let reply = self
            .replies
            .lock()
            .expect("no test should panic while holding the reply lock")
            .pop_front()
            .expect("the test should script a reply for every request");
        let response = ChatMessageResponse {
            model: input.model_name.clone(),
            created_at: String::new(),
            message: ChatMessage::assistant(reply),
            logprobs: None,
            done: true,
            final_data: None,
        };
        async move { Ok(response) }.boxed()
    }
}

impl Drop for ScriptedClient {
    fn drop(&mut self) {
        // A failing test is already unwinding, and a second panic would hide its message.
        if !std::thread::panicking() {
            let replies = self
                .replies
                .get_mut()
                .expect("the reply lock should be intact");
            assert!(replies.is_empty(), "unused scripted replies: {replies:?}");
        }
    }
}

#[tokio::test]
async fn model_proposals_for_each_quote_enter_the_bank() -> anyhow::Result<()> {
    let mut bank = JoinedWordBank::default();
    let client = ScriptedClient::new(&[
        r#"{"joined_words": [{"joined": "theyhad", "split": "they had"}]}"#,
        r#"{"joined_words": [{"joined": "toface", "split": "to face"}]}"#,
    ]);
    let quotes = [(1, "theyhad left"), (2, "nothing toface")];

    add_model_proposals(&mut bank, &quotes, "gemma4:12b-qat", &client).await?;

    let splits: Vec<(&str, &str)> = bank
        .splits
        .iter()
        .map(|(joined, split)| (joined.as_str(), split.as_str()))
        .collect();
    assert_eq!(splits, [("theyhad", "they had"), ("toface", "to face")]);
    Ok(())
}

#[tokio::test]
async fn a_model_is_asked_at_its_default_temperature() -> anyhow::Result<()> {
    let mut bank = JoinedWordBank::default();
    let client = ScriptedClient::new(&[r#"{"joined_words": []}"#]);

    add_model_proposals(&mut bank, &[(1, "theyhad left")], "gemma4:12b-qat", &client).await?;

    let requests = client
        .requests
        .lock()
        .expect("the request lock should be intact");
    let request = serde_json::to_value(&requests[0])?;
    assert_eq!(request["model"], "gemma4:12b-qat");
    // A request leaves out any setting it doesn't make, so the model applies its own default.
    assert!(request["options"]["temperature"].is_null());
    Ok(())
}

#[tokio::test]
async fn a_model_sees_the_quote_as_given() -> anyhow::Result<()> {
    let mut bank = JoinedWordBank::default();
    let client = ScriptedClient::new(&[r#"{"joined_words": []}"#]);

    add_model_proposals(&mut bank, &[(1, "end. Begin")], "gemma4:12b-qat", &client).await?;

    let requests = client
        .requests
        .lock()
        .expect("the request lock should be intact");
    let quote_message = &requests[0]
        .messages
        .last()
        .expect("the request should end with the quote")
        .content;
    assert_eq!(quote_message, r#"{"quote":"end. Begin"}"#);
    Ok(())
}

#[tokio::test]
#[ignore = "needs a running local language model"]
async fn a_live_model_proposes_the_splits_of_joined_words() -> anyhow::Result<()> {
    // The model named by `ACROSTIC_TEST_MODEL`, or else one that fills the bank well.
    let model = env::var("ACROSTIC_TEST_MODEL").unwrap_or("gemma4:12b-qat".to_string());
    let client = BaseClient::new().await?;
    let quote = "build alibrary worthy of your noble soul, andspend without stint";
    let mut bank = JoinedWordBank::default();

    add_model_proposals(&mut bank, &[(1, quote)], &model, &*client).await?;

    assert_eq!(
        bank.split_joined_words(quote),
        "build a library worthy of your noble soul, and spend without stint"
    );
    Ok(())
}
