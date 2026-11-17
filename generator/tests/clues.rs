// Tests for `ClueClient`: prints the clue a language model writes for a word, from the model
// `ACROSTIC_TEST_MODEL` names or else the default, as in
// `ACROSTIC_TEST_MODEL=gemma4:26b-qat cargo test --test clues -- --ignored`. Needs a running
// language model.

use std::{env, mem};

use acrostic_generator::clues::ClueClient;
use acrostic_generator::llm::DEFAULT_MODEL_NAME;
use acrostic_generator::util::interrupt::channel;

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
