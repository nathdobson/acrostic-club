// Tests for the requests `RpcBuilder` sends. Most check a request's contents against a client that
// records it. One test, ignored by default, prints the clues a running language model writes for
// one answer, from the model `ACROSTIC_TEST_MODEL` names or else the default, as in
// `ACROSTIC_TEST_MODEL=gemma4:26b-qat cargo test --test rpcs -- --ignored`.

use std::env;
use std::sync::Mutex;

use acrostic_generator::llm::DEFAULT_MODEL_NAME;
use acrostic_generator::llm::chat_client::{BaseClient, ChatClient};
use acrostic_generator::llm::rpcs::{ClueRequest, ClueResponse, RpcBuilder};
use futures::FutureExt;
use futures::future::BoxFuture;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse};
use serde_json::{Value, json};

// A chat client that records the JSON of each request it receives and answers every one with
// `reply`.
struct RecordingClient {
    reply: String,
    requests: Mutex<Vec<Value>>,
}

impl ChatClient for RecordingClient {
    fn send_chat_messages<'a>(
        &'a self,
        input: &'a ChatMessageRequest,
    ) -> BoxFuture<'a, anyhow::Result<ChatMessageResponse>> {
        async move {
            let request = serde_json::to_value(input)?;
            self.requests
                .lock()
                .expect("no test should panic while holding the lock")
                .push(request);
            Ok(ChatMessageResponse {
                model: input.model_name.clone(),
                created_at: String::new(),
                message: ChatMessage::assistant(self.reply.clone()),
                logprobs: None,
                done: true,
                final_data: None,
            })
        }
        .boxed()
    }
}

// Sends `rpc` to a client that answers with `reply`, returning the parsed reply and the request's
// JSON.
async fn send_recorded(
    rpc: &RpcBuilder<ClueRequest, ClueResponse>,
    reply: &str,
) -> anyhow::Result<(ClueResponse, Value)> {
    let client = RecordingClient {
        reply: reply.to_string(),
        requests: Mutex::new(vec![]),
    };
    let response = rpc.send(&client).await?;
    let mut requests = client
        .requests
        .into_inner()
        .expect("no test should poison the lock");
    let request = requests
        .pop()
        .expect("sending should make exactly one request");
    Ok((response, request))
}

// A request for clues whose answer and count don't matter to the test.
fn build_clue_request() -> anyhow::Result<RpcBuilder<ClueRequest, ClueResponse>> {
    ClueRequest {
        answer: "dog".to_string(),
        clue_count: 1,
    }
    .build()
}

#[tokio::test]
async fn a_request_names_the_default_model() -> anyhow::Result<()> {
    let (_, request) = send_recorded(&build_clue_request()?, r#"{"clues": []}"#).await?;
    assert_eq!(request["model"], json!(DEFAULT_MODEL_NAME));
    Ok(())
}

#[tokio::test]
async fn a_named_model_replaces_the_default() -> anyhow::Result<()> {
    let mut rpc = build_clue_request()?;
    rpc.model("gemma4:26b-qat");

    let (_, request) = send_recorded(&rpc, r#"{"clues": []}"#).await?;

    assert_eq!(request["model"], json!("gemma4:26b-qat"));
    Ok(())
}

#[tokio::test]
async fn a_request_turns_thinking_off() -> anyhow::Result<()> {
    let (_, request) = send_recorded(&build_clue_request()?, r#"{"clues": []}"#).await?;
    assert_eq!(request["think"], json!(false));
    Ok(())
}

#[tokio::test]
async fn a_reply_is_parsed_into_the_response() -> anyhow::Result<()> {
    let (response, _) =
        send_recorded(&build_clue_request()?, r#"{"clues": ["Furry pet."]}"#).await?;
    assert_eq!(response.clues, vec!["Furry pet.".to_string()]);
    Ok(())
}

#[tokio::test]
#[ignore = "needs a running local language model"]
async fn a_clue_request_returns_clues() -> anyhow::Result<()> {
    let model = env::var("ACROSTIC_TEST_MODEL").unwrap_or(DEFAULT_MODEL_NAME.to_string());
    let client = BaseClient::new().await?;
    let answer = "nathan";

    let clues = ClueRequest {
        answer: answer.to_string(),
        clue_count: 30,
    }
    .build()?
    .model(&model)
    .send(&*client)
    .await?;

    println!("{clues:#?}");
    Ok(())
}
