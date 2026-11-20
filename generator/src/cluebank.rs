// The clue bank: every clue written for each answer word, kept in `data/cluebank.json`.
//
// Puzzle files don't carry clue text. Each puzzle entry names its clue by the answer plus a tag,
// and the site build looks up the text when it publishes. A fix to a clue therefore reaches every
// puzzle that uses it. An answer can also hold several clues, so puzzles that share an answer can
// show different clues.
//
// A tag is built from the clue's first few meaningful words (see `tag_for`), so it reads as the
// clue it names. Loading the bank checks every tag against its clue's text, the way a checksum is
// checked, so a tag can't drift onto a different clue unnoticed.
//
// Two alternate tag ideas were considered and discarded: An opaque ID, such as a number or a hash,
// would read as nothing, and each has a further cost. A number can shift onto the wrong clue
// silently. A hash catches drift, but it changes with every edit to the text, even a typo fix, so
// each such edit must update every puzzle that names the clue.
//
// Tags pay a smaller form of that cost. Editing the words a tag is built from changes the tag, so
// every puzzle naming the clue must change along with the bank. `edit_clue` makes both changes
// together. Editing a clue's text by hand in a way that changes its tag makes the next load fail,
// naming the clue.
//
// Each clue also records where it came from and who reviewed it, if anyone, so that tools can tell
// reviewed clues from unreviewed ones. A model's review is preferred to a person's: a person who
// reviews a clue sees its answer, which spoils the puzzles that show the clue.

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow, ensure};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::puzzle::{CLUED_PUZZLE_FILE, read_clued_puzzles};
use crate::{PACKAGE_PATH, read_path_to_string, write_path};

// How many meaningful words of a clue its tag keeps: enough to tell an answer's clues apart, and
// few enough to read at a glance.
const TAG_WORD_COUNT: usize = 3;

// Words a tag skips: they appear in many clues but say little about any one of them.
const FILLER_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "by", "for", "from", "in", "is", "it", "its", "of", "on", "or",
    "that", "the", "this", "to", "which", "who", "with",
];

// The word a tag uses for each ASCII punctuation mark, for a clue with no letters or digits.
const SYMBOL_NAMES: &[(char, &str)] = &[
    ('!', "exclamation"),
    ('"', "quote"),
    ('#', "hash"),
    ('$', "dollar"),
    ('%', "percent"),
    ('&', "ampersand"),
    ('\'', "apostrophe"),
    ('(', "leftparen"),
    (')', "rightparen"),
    ('*', "asterisk"),
    ('+', "plus"),
    (',', "comma"),
    ('-', "hyphen"),
    ('.', "period"),
    ('/', "slash"),
    (':', "colon"),
    (';', "semicolon"),
    ('<', "lessthan"),
    ('=', "equals"),
    ('>', "greaterthan"),
    ('?', "question"),
    ('@', "at"),
    ('[', "leftbracket"),
    ('\\', "backslash"),
    (']', "rightbracket"),
    ('^', "caret"),
    ('_', "underscore"),
    ('`', "backtick"),
    ('{', "leftbrace"),
    ('|', "bar"),
    ('}', "rightbrace"),
    ('~', "tilde"),
];

// Returns where the clue bank lives in the repository.
pub fn clue_bank_path() -> PathBuf {
    PACKAGE_PATH.join("data/cluebank.json")
}

// Builds the tag for a clue's text: its first `TAG_WORD_COUNT` words that aren't filler words,
// lowercased, reduced to ASCII letters and digits, and joined by hyphens. For example, "What
// follows a battle or conflict." gives `what-follows-battle`. Two kinds of clue lack such words:
//
// - A clue made only of filler words, such as "As it is", keeps them instead.
// - A clue with no letters or digits names its first symbols instead (see `symbol_words`), so
//   ":-)" gives `colon-hyphen-rightparen`.
//
// The tags stored in the bank and in the puzzles all come from this function. Changing it,
// `TAG_WORD_COUNT`, `FILLER_WORDS`, or `SYMBOL_NAMES` therefore means retagging the bank and the
// puzzles together.
//
// Fails only when the text is empty or only whitespace, since such a clue shows nothing to name.
fn tag_for(text: &str) -> anyhow::Result<String> {
    // Transliterating before dropping non-ASCII characters keeps accented letters, so "Pokémon"
    // gives `pokemon` rather than `pokmon`.
    let ascii_text = any_ascii::any_ascii(text).to_lowercase();
    let words: Vec<String> = ascii_text
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        let symbol_words = symbol_words(text);
        ensure!(
            !symbol_words.is_empty(),
            "cannot build a tag for clue {text:?}, which is empty or only whitespace"
        );
        return Ok(join_tag_words(&symbol_words));
    }

    let meaningful_words: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|word| !FILLER_WORDS.contains(word))
        .collect();

    // A clue made only of filler words, such as "As it is", still needs a tag.
    let tag_words = if meaningful_words.is_empty() {
        words.iter().map(String::as_str).collect()
    } else {
        meaningful_words
    };
    Ok(join_tag_words(&tag_words))
}

// Names the symbols of a clue that has no letters or digits, one tag word per symbol, such as
// `colon` and `hyphen`. A symbol outside ASCII is named by its ASCII transliteration, so "—" gives
// `hyphen`, and "…" gives `period` three times. A symbol with no ASCII form, such as "𝄞", stands
// for itself.
fn symbol_words(text: &str) -> Vec<String> {
    text.chars()
        .filter(|symbol| !symbol.is_whitespace())
        .flat_map(|symbol| {
            let ascii_form = any_ascii::any_ascii_char(symbol);
            if ascii_form.is_empty() {
                vec![symbol]
            } else {
                ascii_form
                    .chars()
                    .filter(|ascii_symbol| !ascii_symbol.is_whitespace())
                    .collect()
            }
        })
        .map(symbol_word)
        .collect()
}

// Names one symbol: an ASCII punctuation mark by its word in `SYMBOL_NAMES`, and any other symbol
// by its code point, such as `u1d11e` for "𝄞".
fn symbol_word(symbol: char) -> String {
    match SYMBOL_NAMES
        .iter()
        .find(|(named_symbol, _)| *named_symbol == symbol)
    {
        Some((_, name)) => name.to_string(),
        None => {
            let code_point = u32::from(symbol);
            format!("u{code_point:x}")
        }
    }
}

// Joins a tag's first `TAG_WORD_COUNT` words with hyphens.
fn join_tag_words(words: &[impl Borrow<str>]) -> String {
    words[..words.len().min(TAG_WORD_COUNT)].join("-")
}

// Where a clue came from.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClueSource {
    // A language model wrote the clue; the string names the model, such as `phi4`.
    Model(String),
    // The clue came from the xd crossword clue database (https://xd.saul.pw/data).
    Xd,
    // A person wrote the clue.
    Person,
}

// Who reviewed a clue.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reviewer {
    // A language model reviewed the clue; the string names the model, such as `claude-opus-5-5`.
    Model(String),
    // A person reviewed the clue.
    Person,
}

impl fmt::Display for Reviewer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reviewer::Model(model) => formatter.write_str(model),
            Reviewer::Person => formatter.write_str("a person"),
        }
    }
}

// One clue in the bank. Its fields are private so that its tag always matches its text. A field
// the bank doesn't know makes the bank fail to load, rather than vanishing at the next write.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BankedClue {
    tag: String,
    text: String,
    source: ClueSource,
    reviewer: Option<Reviewer>,
}

impl BankedClue {
    // Returns the name puzzles use for this clue, which is always `tag_for(self.text())`.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    // Returns the clue as solvers see it.
    pub fn text(&self) -> &str {
        &self.text
    }

    // Returns where the clue came from.
    pub fn source(&self) -> &ClueSource {
        &self.source
    }

    // Returns who reviewed the clue, or `None` when no one has.
    pub fn reviewer(&self) -> Option<&Reviewer> {
        self.reviewer.as_ref()
    }
}

// Every clue in the bank, by answer. Within each answer the tags are distinct, and every tag
// matches its clue's text.
#[derive(Debug, Default)]
pub struct ClueBank {
    // A sorted map keeps the JSON file's answers in order, so diffs stay small.
    clues_by_answer: BTreeMap<String, Vec<BankedClue>>,
}

impl ClueBank {
    // Reads the bank from its JSON file.
    pub async fn read(path: &Path) -> anyhow::Result<ClueBank> {
        // A failed read becomes a plain message rather than the I/O error itself. `main` skips a
        // puzzle whose stage file is missing by recognizing a not-found I/O error, and a missing
        // bank must not pass for a missing stage file.
        let json = read_path_to_string(path)
            .await
            .map_err(|error| anyhow!("cannot read the clue bank: {error}"))?;
        ClueBank::from_json(&json).with_context(|| format!("cannot load the clue bank at {path:?}"))
    }

    // Writes the bank to its JSON file. The JSON goes to a temporary file that then replaces the
    // bank in one step, so an interrupted write can't leave a cut-off bank behind. Each process
    // gets a temporary file of its own, so when two processes write at once, one process's change
    // can be lost, but their JSON can't mix.
    pub async fn write(&self, path: &Path) -> anyhow::Result<()> {
        let process_id = std::process::id();
        let temporary_path = path.with_extension(format!("json.{process_id}.tmp"));
        let replacement = async {
            write_path(&temporary_path, self.to_json().as_bytes()).await?;
            tokio::fs::rename(&temporary_path, path).await
        }
        .await;
        if let Err(error) = replacement {
            // The temporary file is removed so that none is left behind. The write's own error is
            // the one worth reporting, so a failure to remove the file is ignored.
            let _ = tokio::fs::remove_file(&temporary_path).await;
            return Err(error).with_context(|| format!("cannot replace the clue bank at {path:?}"));
        }
        Ok(())
    }

    // Parses the bank from JSON, checking that every tag matches its clue's text and that no
    // answer has two clues with the same tag.
    pub fn from_json(json: &str) -> anyhow::Result<ClueBank> {
        let clues_by_answer = parse_clues_by_answer(json)
            .context("the clue bank is not in the expected JSON form")?;
        for (answer, clues) in &clues_by_answer {
            for (position, clue) in clues.iter().enumerate() {
                let BankedClue { tag, text, .. } = clue;
                let expected_tag = tag_for(text)?;
                ensure!(
                    *tag == expected_tag,
                    "the clue {text:?} for {answer:?} has tag {tag:?}, but that text calls for \
                     {expected_tag:?}. If the text was edited by hand, restore the text the tag \
                     was built from, then make the change with the `global edit-clue` command, \
                     which also renames the tag in the puzzles that name the clue. If `tag_for` \
                     changed, retag the bank and the puzzles together."
                );
                let is_repeated_tag = clues[..position].iter().any(|earlier| earlier.tag == *tag);
                ensure!(
                    !is_repeated_tag,
                    "{answer:?} has more than one clue tagged {tag:?}"
                );
            }
        }
        Ok(ClueBank { clues_by_answer })
    }

    // Formats the bank as indented JSON, one field per line, so that a fix to a clue changes only
    // the lines of the fields it touches.
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(&self.clues_by_answer)
            .expect("a map from strings to plain clue records should always serialize");
        json.push('\n');
        json
    }

    // Returns the clues for an answer, or an empty slice when the bank has none.
    pub fn clues_for(&self, answer: &str) -> &[BankedClue] {
        self.clues_by_answer.get(answer).map_or(&[], Vec::as_slice)
    }

    // Returns the answer's clue with this tag, if the bank has one.
    pub fn find(&self, answer: &str, tag: &str) -> Option<&BankedClue> {
        self.clues_for(answer).iter().find(|clue| clue.tag == tag)
    }

    // Returns the answer's clue for the puzzle with this index, or `None` when the bank has no
    // clues for the answer. The index picks among the answer's clues, so puzzles that share an
    // answer spread across its clues, while a puzzle picks the same clue on every run as long as
    // the answer's clues stay the same.
    pub fn clue_for_puzzle(&self, answer: &str, puzzle_index: usize) -> Option<&BankedClue> {
        let clues = self.clues_for(answer);
        if clues.is_empty() {
            return None;
        }
        Some(&clues[puzzle_index % clues.len()])
    }

    // Adds an unreviewed clue for an answer and returns it. When the bank already holds this text
    // for the answer, returns that clue instead of adding a copy.
    //
    // Fails when the text's tag already names a different clue for the answer.
    pub fn add(
        &mut self,
        answer: &str,
        text: &str,
        source: ClueSource,
    ) -> anyhow::Result<&BankedClue> {
        let tag = tag_for(text)?;
        let clues = self.clues_by_answer.entry(answer.to_string()).or_default();
        let position = match clues.iter().position(|clue| clue.tag == tag) {
            Some(position) => {
                let existing_text = &clues[position].text;
                ensure!(
                    existing_text == text,
                    "cannot add the clue {text:?} for {answer:?}: its tag {tag:?} already names \
                     the clue {existing_text:?}"
                );
                position
            }
            None => {
                clues.push(BankedClue {
                    tag,
                    text: text.to_string(),
                    source,
                    reviewer: None,
                });
                clues.len() - 1
            }
        };
        Ok(&clues[position])
    }

    // Replaces a clue's text, records `reviewer` as the clue's reviewer, and returns its tag for
    // the new text. A changed text is the reviewer's own, so the clue's source becomes the
    // reviewer; an edit that keeps the text only records the review. The tag changes whenever the
    // edit changes the words the tag is built from, and every puzzle naming the clue must then
    // switch to the new tag. `edit_clue` updates the bank and those puzzles together.
    //
    // Fails when the answer has no clue with this tag, or when the new tag already names another
    // of the answer's clues.
    pub fn edit(
        &mut self,
        answer: &str,
        tag: &str,
        new_text: &str,
        reviewer: Reviewer,
    ) -> anyhow::Result<String> {
        let new_tag = tag_for(new_text)?;
        let clues = self
            .clues_by_answer
            .get_mut(answer)
            .ok_or_else(|| anyhow!("the clue bank has no clues for {answer:?}"))?;
        let position = clues
            .iter()
            .position(|clue| clue.tag == tag)
            .ok_or_else(|| anyhow!("{answer:?} has no clue tagged {tag:?}"))?;
        let is_taken_by_another_clue = clues
            .iter()
            .enumerate()
            .any(|(other, clue)| other != position && clue.tag == new_tag);
        ensure!(
            !is_taken_by_another_clue,
            "cannot change the clue {tag:?} for {answer:?} to {new_text:?}: that text's tag \
             {new_tag:?} already names another clue"
        );

        let clue = &mut clues[position];
        if clue.text != new_text {
            clue.source = match &reviewer {
                Reviewer::Model(model) => ClueSource::Model(model.clone()),
                Reviewer::Person => ClueSource::Person,
            };
        }
        clue.tag = new_tag.clone();
        clue.text = new_text.to_string();
        clue.reviewer = Some(reviewer);
        Ok(new_tag)
    }
}

// Parses the bank's JSON into its map from answers to clues. Parsing straight into a map would
// silently keep only the last listing of an answer that appears twice, as a careless merge can
// leave. A repeated answer fails to parse instead.
fn parse_clues_by_answer(json: &str) -> serde_json::Result<BTreeMap<String, Vec<BankedClue>>> {
    // Visits the JSON object's entries one at a time, so that a repeated answer can be caught.
    struct AnswersVisitor;

    impl<'de> Visitor<'de> for AnswersVisitor {
        type Value = BTreeMap<String, Vec<BankedClue>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an object mapping each answer to its clues")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<Self::Value, A::Error> {
            let mut clues_by_answer = BTreeMap::new();
            while let Some((answer, clues)) = entries.next_entry::<String, Vec<BankedClue>>()? {
                if clues_by_answer.contains_key(&answer) {
                    let message = format!("the answer {answer:?} appears more than once");
                    return Err(de::Error::custom(message));
                }
                clues_by_answer.insert(answer, clues);
            }
            Ok(clues_by_answer)
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(json);
    let clues_by_answer = serde::Deserializer::deserialize_map(&mut deserializer, AnswersVisitor)?;
    // Rejects anything after the object, as `serde_json::from_str` does.
    deserializer.end()?;
    Ok(clues_by_answer)
}

// Changes a clue's text in the bank, recording `reviewer` as the clue's reviewer, and, when that
// changes the clue's tag, renames the tag in every puzzle that names the clue.
pub async fn edit_clue(
    answer: &str,
    tag: &str,
    new_text: &str,
    reviewer: Reviewer,
) -> anyhow::Result<()> {
    let bank_path = clue_bank_path();
    let mut bank = ClueBank::read(&bank_path).await?;
    let new_tag = bank.edit(answer, tag, new_text, reviewer.clone())?;

    // Every affected puzzle is read and retagged before any file is written, so a puzzle that
    // fails to read leaves every file as it was.
    let mut retagged_puzzles = vec![];
    if new_tag != tag {
        for (index, mut puzzle) in read_clued_puzzles().await? {
            if puzzle.retag_clue(answer, tag, &new_tag) {
                retagged_puzzles.push((index, puzzle));
            }
        }
    }

    // The puzzles are written before the bank. If writing stops between two puzzle files, the bank
    // still holds the old tag, so rerunning the same edit retags the puzzles that remain.
    for (index, puzzle) in &retagged_puzzles {
        puzzle.write(*index, CLUED_PUZZLE_FILE).await?;
    }
    bank.write(&bank_path).await?;

    // Only now is the change complete, so only now is it reported.
    println!(
        "saved the clue {tag:?} for {answer:?} as reviewed by {reviewer}, with text {new_text:?} \
         and tag {new_tag:?}"
    );
    for (index, _) in &retagged_puzzles {
        println!("puzzle {index}: retagged {answer:?} from {tag:?} to {new_tag:?}");
    }
    Ok(())
}
