// Tests for `ClueRequest`: prints the clues a language model writes for one answer. Needs a
// running language model.

use acrostic_generator::llm::chat_client::BaseClient;
use acrostic_generator::llm::rpcs::ClueRequest;

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    // let ollama = Ollama::default();
    let client = BaseClient::new().await?;
    let answer = "nathan";
    let clues = ClueRequest {
        answer: answer.to_string(),
        clue_count: 30,
    }
    .build()?
    .send(&*client)
    .await?;
    println!("{:#?}", clues);
    Ok(())
}
