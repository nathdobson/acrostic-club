// Tests for `ClueClient`: prints the clue a language model writes for a word. Needs a running
// language model.

use std::mem;
use std::time::Instant;

use acrostic_generator::clues::ClueClient;
use acrostic_generator::util::interrupt::channel;

#[tokio::test]
async fn test_clue_client() -> anyhow::Result<()> {
    let (tx, mut rx) = channel();
    let client = ClueClient::new(tx).await?;
    {
        let start = Instant::now();
        for word in &[
            // "extant", "netball",
            // "nathan",
            // "andrew", "john", "hindwings"
            // "dudley",
            // "atkinson",
            // "howell",
            "india's",
        ] {
            let clues = client.create_clue(word).await?;
            println!("{:?}", clues);
        }
    }
    mem::drop(client);
    rx.cleanup().await?;
    Ok(())
}
