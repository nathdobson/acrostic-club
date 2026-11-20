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
// every puzzle naming the clue must change along with the bank. `edit_clues` makes both changes
// together. Editing a clue's text by hand in a way that changes its tag makes the next load fail,
// naming the clue.
//
// Each clue also records where it came from and, once someone has reviewed it, the review: who
// reviewed it, a verdict, and any problems found. New puzzles prefer an answer's best-reviewed
// clues (see `ClueBank::clue_for_puzzle`), and the problems tell a later rewriter what to fix. A
// review covers only the text it judged, so changing a clue's text drops its review. A person's
// review is more reliable than a model's, but a model's review avoids spoilers: a person who
// reviews a clue sees its answer, which spoils the puzzles that use the clue.

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

// A reviewer's judgment of a clue, on the scale the clue-grading prompt uses.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    // The clue is inaccurate or unfair, or it gives the answer away.
    Flawed,
    // The clue is accurate and fair, but offers nothing beyond a plain definition or fact.
    Flat,
    // The clue is accurate and fair, and delivers something more, such as wordplay, misdirection,
    // or a moment of delight. A person's own edit of a clue records this verdict too, whatever the
    // clue delivers (see `ClueEdit::by_person`): new puzzles should prefer a clue a person chose,
    // and later upgrades should leave it alone.
    Satisfying,
}

// One reviewer's judgment of a clue's text, such as a `Flat` verdict with the problem "The clue is
// a dictionary definition."
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub reviewer: Reviewer,
    pub verdict: Verdict,
    // Each problem the reviewer found, in a sentence of its own. A clue of any verdict may have
    // problems: a flat clue's problems often say why it falls short of satisfying.
    pub problems: Vec<String>,
}

// A clue's rank among its answer's clues: a new puzzle picks only from the highest-ranked ones. The
// ranks run from lowest to highest. An unreviewed clue ranks below a flat one, which a review
// found accurate and fair, and above a flawed one, which a review found wanting: until someone
// reviews it, it could prove either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ClueRank {
    Flawed,
    Unreviewed,
    Flat,
    Satisfying,
}

// One clue in the bank. Its fields are private so that its tag always matches its text. A field
// the bank doesn't know makes the bank fail to load, rather than vanishing at the next write.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BankedClue {
    tag: String,
    text: String,
    source: ClueSource,
    review: Option<Review>,
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

    // Returns the review of the clue's current text, or `None` when no one has reviewed it.
    pub fn review(&self) -> Option<&Review> {
        self.review.as_ref()
    }

    // Returns the clue's rank among its answer's clues, which the verdict of its review sets.
    fn rank(&self) -> ClueRank {
        match &self.review {
            None => ClueRank::Unreviewed,
            Some(review) => match review.verdict {
                Verdict::Flawed => ClueRank::Flawed,
                Verdict::Flat => ClueRank::Flat,
                Verdict::Satisfying => ClueRank::Satisfying,
            },
        }
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
                     was built from, then make the change through `edit_clues`, which also \
                     renames the tag in the puzzles that name the clue; a person can run it as \
                     `global edit-clue-as-person`. If `tag_for` changed, retag the bank and the \
                     puzzles together."
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
    // clues for the answer. Only the answer's best-ranked clues are candidates: its satisfying
    // clues if it has any, else its flat ones, else its unreviewed ones, else its flawed ones. The
    // index picks among the candidates, so puzzles that share an answer spread across them, while
    // a puzzle picks the same clue on every run as long as the answer's clues and their reviews
    // stay the same.
    pub fn clue_for_puzzle(&self, answer: &str, puzzle_index: usize) -> Option<&BankedClue> {
        let clues = self.clues_for(answer);
        let best_rank = clues.iter().map(BankedClue::rank).max()?;
        let candidates: Vec<&BankedClue> = clues
            .iter()
            .filter(|clue| clue.rank() == best_rank)
            .collect();
        Some(candidates[puzzle_index % candidates.len()])
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
                    review: None,
                });
                clues.len() - 1
            }
        };
        Ok(&clues[position])
    }

    // Replaces a clue's text, credits the new text to `source`, and returns the clue's tag for the
    // new text. A review covers only the text it judged, so the edit drops the clue's review, and
    // `review` can record one for the new text. Text that matches the clue's current text changes
    // nothing, so the clue keeps its source and review. The tag changes whenever the edit changes
    // the words the tag is built from, and every puzzle naming the clue must then switch to the
    // new tag. `edit_clues` updates the bank and those puzzles together.
    //
    // Fails when the answer has no clue with this tag, or when the new tag already names another
    // of the answer's clues.
    pub fn edit(
        &mut self,
        answer: &str,
        tag: &str,
        new_text: &str,
        source: ClueSource,
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
            clue.tag = new_tag.clone();
            clue.text = new_text.to_string();
            clue.source = source;
            clue.review = None;
        }
        Ok(new_tag)
    }

    // Records `review` as the review of the answer's clue with this tag, replacing any earlier
    // review.
    //
    // Fails when the answer has no clue with this tag.
    pub fn review(&mut self, answer: &str, tag: &str, review: Review) -> anyhow::Result<()> {
        let clue = self
            .clues_by_answer
            .get_mut(answer)
            .and_then(|clues| clues.iter_mut().find(|clue| clue.tag == tag))
            .ok_or_else(|| anyhow!("{answer:?} has no clue tagged {tag:?}"))?;
        clue.review = Some(review);
        Ok(())
    }
}

// One change to a clue's text, which `edit_clues` applies along with any review of the new text.
// When the new text matches the clue's current text, only the review is recorded, and the clue
// keeps its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClueEdit {
    pub answer: String,
    // The tag of the clue to change.
    pub tag: String,
    pub new_text: String,
    // Who wrote the new text.
    pub source: ClueSource,
    // The review of the new text, or `None` to leave a changed clue unreviewed.
    pub review: Option<Review>,
}

impl ClueEdit {
    // Returns a person's edit of a clue: the new text is credited to a person, along with that
    // person's review, which has a `Satisfying` verdict and no problems. Text that matches the
    // clue's current text approves the clue as it stands. `global edit-clue-as-person` makes this
    // edit.
    pub fn by_person(answer: &str, tag: &str, new_text: &str) -> ClueEdit {
        ClueEdit {
            answer: answer.to_string(),
            tag: tag.to_string(),
            new_text: new_text.to_string(),
            source: ClueSource::Person,
            review: Some(Review {
                reviewer: Reviewer::Person,
                verdict: Verdict::Satisfying,
                problems: vec![],
            }),
        }
    }
}

// A change to a clue's tag, which every puzzle naming the clue must follow.
struct TagRename {
    answer: String,
    old_tag: String,
    new_tag: String,
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

// Applies `edits` to the bank in order, each with its review, and renames each changed tag in every
// puzzle that names the clue. A later edit of a clue that an earlier edit retagged names the clue
// by its new tag. A model's edits come here directly, crediting the model as their source; a
// person's come through `global edit-clue-as-person`.
pub async fn edit_clues(edits: &[ClueEdit]) -> anyhow::Result<()> {
    let bank_path = clue_bank_path();
    let mut bank = ClueBank::read(&bank_path).await?;
    let mut new_tags = vec![];
    let mut renames = vec![];
    for edit in edits {
        let new_tag = bank.edit(&edit.answer, &edit.tag, &edit.new_text, edit.source.clone())?;
        if let Some(review) = &edit.review {
            bank.review(&edit.answer, &new_tag, review.clone())?;
        }
        if new_tag != edit.tag {
            renames.push(TagRename {
                answer: edit.answer.clone(),
                old_tag: edit.tag.clone(),
                new_tag: new_tag.clone(),
            });
        }
        new_tags.push(new_tag);
    }

    // Every affected puzzle is read and retagged before any file is written, so a puzzle that
    // fails to read leaves every file as it was.
    let mut retagged_puzzles = vec![];
    if !renames.is_empty() {
        for (index, mut puzzle) in read_clued_puzzles().await? {
            // The renames apply in order, so a puzzle follows a clue through several edits.
            let mut applied_renames = vec![];
            for rename in &renames {
                if puzzle.retag_clue(&rename.answer, &rename.old_tag, &rename.new_tag) {
                    applied_renames.push(rename);
                }
            }
            if !applied_renames.is_empty() {
                retagged_puzzles.push((index, puzzle, applied_renames));
            }
        }
    }

    // The puzzles are written before the bank. If writing stops between two puzzle files, the bank
    // still holds the old tags, so rerunning the same edits retags the puzzles that remain.
    for (index, puzzle, _) in &retagged_puzzles {
        puzzle.write(*index, CLUED_PUZZLE_FILE).await?;
    }
    bank.write(&bank_path).await?;

    // Only now is the change complete, so only now is it reported.
    for (edit, new_tag) in edits.iter().zip(&new_tags) {
        let ClueEdit {
            answer,
            tag,
            new_text,
            ..
        } = edit;
        println!(
            "saved the clue {tag:?} for {answer:?} with text {new_text:?} and tag {new_tag:?}"
        );
    }
    for (index, _, applied_renames) in &retagged_puzzles {
        for TagRename {
            answer,
            old_tag,
            new_tag,
        } in applied_renames
        {
            println!("puzzle {index}: retagged {answer:?} from {old_tag:?} to {new_tag:?}");
        }
    }
    Ok(())
}
