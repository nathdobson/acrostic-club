use crate::llm::DEFAULT_MODEL_NAME;
use crate::llm::chat_client::ChatClient;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::{ChatMessage, MessageRole};
use ollama_rs::generation::parameters::{FormatType, JsonSchema, JsonStructure};
use ollama_rs::models::ModelOptions;
use serde::{Deserialize, Serialize};

// A request to a language model, assembled one setting at a time and then sent with `send`. Each
// setter, such as `seed`, stores its setting and returns the builder, so that calls can chain, as
// in `rpc.seed(seed).model(name)`.
pub struct RpcBuilder<Req, Resp> {
    req: Req,
    training: Vec<(Req, Resp)>,
    system: String,
    seed: i32,
    model: String,
}

impl<Req: Serialize, Resp: JsonSchema + Serialize + for<'de> Deserialize<'de>>
    RpcBuilder<Req, Resp>
{
    pub fn new(req: Req, system: String) -> anyhow::Result<Self> {
        Ok(RpcBuilder {
            req,
            training: vec![],
            system,
            seed: 123665,
            model: DEFAULT_MODEL_NAME.to_string(),
        })
    }
    // Sets the seed for the model's random choices. The model scores every candidate token and
    // draws the next one at random, favoring higher scores, and a fixed seed makes the same draws
    // each time.
    //
    // The same request can still get a different reply. The scores are floating-point sums, whose
    // rounding depends on how Ollama arranges the work: how many requests it runs together, whether
    // it reuses the start of an earlier prompt, the hardware, and Ollama's own version. A shifted
    // score can change which token a draw lands on, and the rest of the reply follows from that
    // token. Setting the temperature to 0, which removes the draws, wouldn't help either: rounding
    // can still reorder two nearly tied tokens. The replies `CacheClient` stores, not the seed, are
    // what let a rerun reproduce a reply.
    pub fn seed(&mut self, seed: i32) -> &mut Self {
        self.seed = seed;
        self
    }
    // Sets the Ollama model that answers the request. Without this call, the request goes to the
    // default model that `new` sets.
    pub fn model(&mut self, model: &str) -> &mut Self {
        self.model = model.to_string();
        self
    }
    pub fn train(&mut self, req: Req, resp: Resp) -> &mut Self {
        self.training.push((req, resp));
        self
    }
    pub async fn send(&self, ollama: &dyn ChatClient) -> anyhow::Result<Resp> {
        let mut messages = vec![];
        messages.push(ChatMessage::new(MessageRole::System, self.system.clone()));
        for (req, resp) in &self.training {
            messages.push(ChatMessage::new(
                MessageRole::User,
                serde_json::to_string(req)?,
            ));
            messages.push(ChatMessage::new(
                MessageRole::Assistant,
                serde_json::to_string(resp)?,
            ));
        }
        messages.push(ChatMessage::new(
            MessageRole::User,
            serde_json::to_string(&self.req)?,
        ));
        // Ollama constrains the reply to the JSON schema of `Resp`.
        let reply_format = FormatType::StructuredJson(Box::new(JsonStructure::new::<Resp>()));
        let resp = ollama
            .send_chat_messages(
                &ChatMessageRequest::new(self.model.clone(), messages)
                    .options(ModelOptions::default().seed(self.seed))
                    .format(reply_format)
                    // Models that can think, such as Gemma 4, otherwise do so before every reply,
                    // which can stretch a short request to minutes. Ollama accepts `false` from
                    // models that can't think.
                    .think(false),
            )
            .await?;
        let resp = serde_json::from_str(&resp.message.content)?;
        Ok(resp)
    }
}

#[derive(Serialize)]
pub struct ClueRequest {
    pub answer: String,
    pub clue_count: usize,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct ClueResponse {
    pub clues: Vec<String>,
}

impl ClueRequest {
    pub fn build(self) -> anyhow::Result<RpcBuilder<ClueRequest, ClueResponse>> {
        let mut rpc = RpcBuilder::new(
            self,
            "You are a crossword clue generator. \
                        You generate several diverse crossword clues for a given answer."
                .to_string(),
        )?;
        rpc.train(
            ClueRequest {
                answer: "dog".to_string(),
                clue_count: 2,
            },
            ClueResponse {
                clues: vec!["Furry pet".to_string(), "Man's best friend".to_string()],
            },
        );
        rpc.train(
            ClueRequest {
                answer: "einstein".to_string(),
                clue_count: 3,
            },
            ClueResponse {
                clues: vec![
                    "Albert of physics fame".to_string(),
                    "He postulated E=mc^2".to_string(),
                    "Eponym of genius".to_string(),
                ],
            },
        );
        rpc.train(
            ClueRequest {
                answer: "pitt".to_string(),
                clue_count: 1,
            },
            ClueResponse {
                clues: vec!["Brad ____ from the silver screen".to_string()],
            },
        );
        Ok(rpc)
    }
}

#[derive(Serialize)]
pub struct AnswerRequest {
    pub clue: String,
    pub letter_count: usize,
    pub answer_count: usize,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct AnswerResponse {
    pub answers: Vec<String>,
}

impl AnswerRequest {
    pub fn build(self) -> anyhow::Result<RpcBuilder<AnswerRequest, AnswerResponse>> {
        RpcBuilder::new(
            self,
            "You are a crossword clue solver. You provide several possible answers for a crossword clue. You do not use any forms of the answer word in the clue.".to_string(),
        )
    }
}

// Asks for the words that a quote runs together with nothing between them, as in "theyhad".
#[derive(Serialize)]
pub struct JoinedWordsRequest {
    pub quote: String,
}

// The joined words the model finds in the quote, which may be none.
#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct JoinedWordsResponse {
    pub joined_words: Vec<JoinedWord>,
}

// A word of a quote that the model reads as several words run together.
#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JoinedWord {
    // The word as the model spells it, such as "theyhad". The prompt asks the model to copy the
    // word from the quote, but a model doesn't always copy it exactly. It may straighten a curly
    // apostrophe, change a letter's case, keep punctuation from either end, or misspell the word.
    // `JoinedWordBank::add_proposal` checks the word against the quote, and the bank keeps the
    // quote's spelling.
    pub joined: String,
    // The words it splits into, separated by spaces, such as "they had".
    pub split: String,
}

impl JoinedWordsRequest {
    // Builds the request for `model`.
    pub fn build(
        self,
        model: &str,
    ) -> anyhow::Result<RpcBuilder<JoinedWordsRequest, JoinedWordsResponse>> {
        let mut rpc = RpcBuilder::new(
            self,
            "Some words in this quote run together because line breaks or spaces were lost, as \
             in \"theyhad\" for \"they had\". List each run-together word exactly as the quote \
             spells it, with the words it splits into. Leave out words that are spelled \
             correctly as one word, such as \"cannot\" or \"someone\". If no words run \
             together, reply with an empty list. Only add spaces; never fix spelling or typos, \
             such as \"teh\" or \"th\" for \"the\"."
                .to_string(),
        )?;
        rpc.model(model);
        Ok(rpc)
    }
}
