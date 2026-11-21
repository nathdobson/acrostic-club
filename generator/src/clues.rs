use crate::cluebank::{BankedClue, ClueBank, ClueSource, clue_bank_path};
use crate::cluedb::{CLUE_DB, ClueDb};
use crate::lemma::{LEMMA, Lemma};
use crate::llm::chat_client::ChatClient;
use crate::llm::new_client;
use crate::llm::rpcs::{AnswerRequest, ClueRequest};
use crate::ontology::{ONTOLOGY, Ontology};
use anyhow::ensure;
use ordered_float::NotNan;
use std::path::Path;
use std::sync::Arc;

use crate::puzzle::{CLUED_PUZZLE_FILE, Puzzle};
use crate::string::LetterString;
use crate::subseq::longest_subsequence;
use crate::util::interrupt::CleanupSender;
use crate::util::lazy_async::CloneError;

pub struct ClueClient {
    client: Arc<dyn ChatClient>,
    // The Ollama model that writes and checks clues.
    model: String,
    ontology: Arc<Ontology>,
    lemma: Arc<Lemma>,
    clue_db: Arc<ClueDb>,
}

impl ClueClient {
    pub async fn new(cleanup: CleanupSender, model: &str) -> anyhow::Result<Self> {
        let client = new_client(cleanup).await?;
        Ok(ClueClient {
            client,
            model: model.to_string(),
            ontology: ONTOLOGY.get().await.clone_error_static()?.clone(),
            lemma: LEMMA.get().await.clone_error_static()?.clone(),
            clue_db: CLUE_DB.get().await.clone_error_static()?.clone(),
        })
    }
    pub fn score(&self, word: &str, clue: &str) -> Option<NotNan<f64>> {
        let word_letters = LetterString::from_str(word);
        let clue_letters = LetterString::from_str(clue);
        let mut is_banned = false;

        for banned in self
            .lemma
            .alternates(word)
            .iter()
            .chain(self.lemma.canonicals(word).iter())
            .chain(self.ontology.get_conflicts(word).iter())
        {
            let banned_letters = LetterString::from_str(&banned);
            if banned_letters.len() >= 3 {
                if clue_letters
                    .windows(banned_letters.len())
                    .any(|x| x == &*banned_letters)
                {
                    // eprintln!("clue {:?} contains {:?} which is banned for {:?}", clue, banned,
                    // word);
                    is_banned = true;
                }
            }
        }
        if is_banned {
            None
        } else {
            Some(
                -(NotNan::new(longest_subsequence(&word_letters, &clue_letters) as f64)
                    .expect("a length should never be NaN")),
            )
        }
    }
    // Returns the tag of a clue for `answer` in the puzzle with index `puzzle_index`: one of
    // `bank`'s clues when it has any, picked by `ClueBank::clue_for_puzzle`, or else a new clue,
    // which is written to the bank file at `bank_path` right away. Returns `None` when the bank has
    // no clue for the answer and `create_clue` finds none either.
    pub async fn choose_clue(
        &self,
        answer: &str,
        puzzle_index: usize,
        bank: &mut ClueBank,
        bank_path: &Path,
    ) -> anyhow::Result<Option<String>> {
        if let Some(banked_clue) = bank.clue_for_puzzle(answer, puzzle_index) {
            return Ok(Some(banked_clue.tag().to_string()));
        }
        let Some((text, source)) = self.create_clue(answer).await? else {
            return Ok(None);
        };

        // The model calls above can take minutes. Rereading the bank before adding the new clue
        // keeps any change made to the file meanwhile, such as by `edit_clues`. Writing right away
        // keeps the clue even if a later answer fails.
        *bank = ClueBank::read(bank_path).await?;
        let tag = bank.add(answer, &text, source)?.tag().to_string();
        bank.write(bank_path).await?;
        Ok(Some(tag))
    }

    // Finds a new clue for `answer` and reports its source. A clue from the model must pass two
    // checks: it avoids words related to `answer`, and the model, asked to solve it, names `answer`
    // among its guesses. When no clue from the model passes, the xd database's first clue for
    // `answer` stands in. Either way, a clue with no letters or digits is skipped (see
    // `has_letters_or_digits`).
    pub async fn create_clue(&self, answer: &str) -> anyhow::Result<Option<(String, ClueSource)>> {
        println!("creating clue for `{answer}`");
        for seed in 0..10 {
            let clues = ClueRequest {
                answer: answer.to_string(),
                clue_count: 10,
            }
            .build()?
            .seed(123455454 + seed)
            .model(&self.model)
            .send(&*self.client)
            .await?
            .clues;
            let mut clues = clues
                .into_iter()
                .filter(|clue| has_letters_or_digits(clue))
                .filter_map(|clue| {
                    let score = self.score(&answer, &clue)?;
                    Some((clue, score))
                })
                .collect::<Vec<_>>();
            clues.sort_by_key(|x| -x.1);
            let clues = clues
                .into_iter()
                .map(|(clue, _score)| clue)
                .collect::<Vec<_>>();
            println!("    candidate clues {clues:?}");
            for clue in clues {
                println!("    candidate clue {clue}");
                let answers = AnswerRequest {
                    clue: clue.clone(),
                    letter_count: answer.len(),
                    answer_count: 10,
                }
                .build()?
                .model(&self.model)
                .send(&*self.client)
                .await?
                .answers;
                println!("        candidate answers {answers:?}");
                if answers.iter().any(|x| {
                    contains_subsequence(
                        &*LetterString::from_str(x),
                        &*LetterString::from_str(answer),
                    )
                }) {
                    println!("       Done! `{answer}` <= `{clue}`");
                    return Ok(Some((clue, ClueSource::Model(self.model.clone()))));
                }
            }
        }
        let xd_entries = self.clue_db.lookup(&LetterString::from_str(answer));
        if let Some(entry) = xd_entries
            .iter()
            .find(|entry| has_letters_or_digits(&entry.clue))
        {
            println!("       Used backup database for {answer}: {}", entry.clue);
            return Ok(Some((entry.clue.clone(), ClueSource::Xd)));
        }
        println!("       Failed to generate clue for {answer}");
        Ok(None)
    }
}

fn contains_subsequence<T: Eq>(haystack: &[T], needle: &[T]) -> bool {
    haystack.windows(needle.len()).any(|x| x == needle)
}

// Returns whether a clue has any letters or digits. The chat step skips clues that don't, since
// most such clues in the xd database are placeholders, such as "-" beside a fragment of a theme
// entry.
fn has_letters_or_digits(clue: &str) -> bool {
    clue.chars().any(char::is_alphanumeric)
}

// Chooses a clue for each of a puzzle's answers and, once every answer has one, writes the puzzle's
// stage-3 file. An answer the clue bank already covers takes a clue from the bank; any other answer
// gets a new clue, which joins the bank right away.
//
// Adding each new clue rereads and rewrites the whole bank, so when two commands write the bank at
// the same moment, the later write drops the other command's change. `main` runs this step one
// puzzle at a time, so the step never races itself. A tag that `edit_clues` renames while this step
// runs can still go stale in the puzzle being written; the site build then reports it.
pub async fn add_chat(pindex: usize, client: &ClueClient) -> anyhow::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage2.json").await?;
    let bank_path = clue_bank_path();
    let mut bank = ClueBank::read(&bank_path).await?;
    let entries = puzzle
        .clues
        .as_mut()
        .expect("a puzzle's stage-2 file should hold its answers");
    for entry in entries.iter_mut() {
        entry.clue_tag = client
            .choose_clue(&entry.answer, pindex, &mut bank, &bank_path)
            .await?;
    }

    let answers_without_clues: Vec<&str> = entries
        .iter()
        .filter(|entry| entry.clue_tag.is_none())
        .map(|entry| entry.answer.as_str())
        .collect();
    ensure!(
        answers_without_clues.is_empty(),
        "found no clue for {answers_without_clues:?}"
    );
    puzzle.write(pindex, CLUED_PUZZLE_FILE).await?;
    Ok(())
}

// Writes new clues for answers. `ClueClient` writes them with a language model; the dry run takes
// any writer, so a test can supply scripted clues in place of a model's.
pub trait ClueWriter {
    // Writes a new clue for `answer` and reports where it came from, or returns `None` when no clue
    // can be found. The future is `Send`, so the chat stage can run it on Tokio's multi-threaded
    // runtime.
    fn write_clue(
        &self,
        answer: &str,
    ) -> impl Future<Output = anyhow::Result<Option<(String, ClueSource)>>> + Send;
}

impl ClueWriter for ClueClient {
    async fn write_clue(&self, answer: &str) -> anyhow::Result<Option<(String, ClueSource)>> {
        self.create_clue(answer).await
    }
}

// Shows how `client`'s model would clue a puzzle's answers, beside the clues the clue bank holds
// for them, and saves nothing to the bank or the puzzle, so models can be compared without their
// clues joining the bank. The reply cache still records the model's replies, so later runs with the
// same model write the same clues.
pub async fn preview_chat(puzzle_index: usize, client: &ClueClient) -> anyhow::Result<()> {
    let puzzle = Puzzle::read(puzzle_index, "stage2.json").await?;
    let bank = ClueBank::read(&clue_bank_path()).await?;
    let preview = preview_clues(&puzzle, &bank, client).await?;

    // The preview prints after `create_clue`'s progress lines for every answer.
    let model = &client.model;
    println!("puzzle={puzzle_index}: clues from {model} beside the clue bank's");
    print!("{preview}");
    Ok(())
}

// Writes a new clue for each of `puzzle`'s answers with `writer`, even for an answer `bank` already
// covers, and lays each out below the bank's clues for its answer, as in:
//
// ```text
// argent
//     bank: Silver, in heraldry  [xd]
//     new:  Heraldic silver  [gemma4:31b-qat]
// ```
//
// Each clue is followed by where it came from, and "(none)" stands in for a missing clue on either
// side. The puzzle and the bank are only read, so the preview changes nothing in either.
pub async fn preview_clues(
    puzzle: &Puzzle,
    bank: &ClueBank,
    writer: &impl ClueWriter,
) -> anyhow::Result<String> {
    let entries = puzzle
        .clues
        .as_ref()
        .expect("a puzzle at the chat stage should hold its answers");
    let mut preview = String::new();
    for entry in entries {
        let answer = &entry.answer;
        let new_clue = writer.write_clue(answer).await?;
        preview += &format_clue_comparison(answer, bank.clues_for(answer), new_clue.as_ref());
    }
    Ok(preview)
}

// Lays out one answer for comparing its banked clues with its new one. `preview_clues` shows a full
// example.
fn format_clue_comparison(
    answer: &str,
    banked_clues: &[BankedClue],
    new_clue: Option<&(String, ClueSource)>,
) -> String {
    let mut lines = vec![answer.to_string()];
    if banked_clues.is_empty() {
        lines.push("    bank: (none)".to_string());
    }
    for banked_clue in banked_clues {
        let text = banked_clue.text();
        let source = source_label(banked_clue.source());
        lines.push(format!("    bank: {text}  [{source}]"));
    }
    let new_line = match new_clue {
        Some((text, source)) => {
            let source = source_label(source);
            format!("    new:  {text}  [{source}]")
        }
        None => "    new:  (none)".to_string(),
    };
    lines.push(new_line);
    lines.join("\n") + "\n"
}

// Labels where a clue came from, for the dry run's comparison.
fn source_label(source: &ClueSource) -> &str {
    match source {
        ClueSource::Model(model) => model,
        ClueSource::Xd => "xd",
        ClueSource::Person => "person",
    }
}
