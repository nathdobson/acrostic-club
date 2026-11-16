// Tests for `ClueClient`: prints the clue a language model writes for a word. Needs a running
// language model.

use std::mem;

use acrostic_generator::clues::ClueClient;
use acrostic_generator::util::interrupt::channel;

#[tokio::test]
async fn a_clue_client_writes_a_clue() -> anyhow::Result<()> {
    let (cleanup_sender, cleanup_receiver) = channel();
    let client = ClueClient::new(cleanup_sender).await?;

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
