// The joined-word bank lists words that the quote list runs together with nothing between them,
// such as "theyhad". Each comes with the split that restores its lost spaces. The bank lives in
// `data/joined_words.toml`.
//
// The quote list lost many spaces before it reached this repository. Stage 0 restores many of them
// programmatically, such as when punctuation hints at a dropped space, as in "close.If". Where two
// words are just joined together, splitting them relies on a language model (LLM). The models'
// judgments go into the bank rather than straight into puzzles. Building a puzzle then reads a
// tracked file instead of asking a model, so it gives the same quote on any machine.
//
// `global joined-words --model <name>` fills the bank. It asks a model about every quote that fits
// a puzzle, or only those given by puzzle number, and adds what the model finds. After the first
// fill, a run is needed only for the puzzles whose quotes stage 0 newly accepts or rewrites, such
// as when its length limits or repairs change.
//
// Words that look joined but aren't go in a blocklist of words to keep whole, and no proposal may
// split them. Some typos – such as "anther" for "another" – also belong there, since local LLMs can
// be overzealous.
//
// Every committed entry is applied directly to rewrite puzzle text. A person must therefore check
// each new entry before committing the bank, and move any wrong one to the words to keep whole. A
// model's wrong split would otherwise reach every puzzle that holds the word.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, ensure};
use regex::{NoExpand, Regex};
use serde::Deserialize;
use toml_writer::{ToTomlKey, ToTomlValue};

use crate::llm::chat_client::ChatClient;
use crate::llm::rpcs::{JoinedWord, JoinedWordsRequest};
use crate::quote::QuoteCatalog;
use crate::{PACKAGE_PATH, read_path_to_string, write_path};

// Returns where the joined-word bank lives in the repository.
pub fn joined_word_bank_path() -> PathBuf {
    PACKAGE_PATH.join("data/joined_words.toml")
}

// The comment that opens the bank's file, describing its fields for a person editing it. `to_toml`
// writes the comment each time, since rewriting the file drops any other comments.
const FILE_HEADER: &str = "\
# The joined-word bank: words that the quote list runs together with nothing between them, and how
# to split them. Puzzles split these words in their quotes, so a person must check each new entry
# before committing this file. `global joined-words` rewrites the file, dropping any comment but
# this one.
#
# keep_whole: words that look joined but aren't, such as \"doorjamb\". No split may break them. Move
# any wrong entry from `splits` here.
#
# splits: each joined word as the quotes spell it, such as \"theyhad\", mapped to the same text with a
# space at each break, such as \"they had\".
#
# A comment after each word lists the publishable puzzles whose quotes hold it. A publishable puzzle
# is one whose quote stage 0 accepts. Other quotes, such as duplicates and those outside the length
# limits, can't make a puzzle, so an entry that no publishable puzzle holds does nothing for now.
";

// The comment for a word that no publishable puzzle's quote holds.
const NO_PUBLISHABLE_PUZZLE: &str = "no publishable puzzle";

// The bank, read from `data/joined_words.toml`.
#[derive(Deserialize, Debug, Default, PartialEq)]
pub struct JoinedWordBank {
    pub keep_whole: BTreeSet<String>,
    pub splits: BTreeMap<String, String>,
}

// Why a model's proposal was left out of the bank.
#[derive(Debug, PartialEq)]
pub enum ProposalRejection {
    // The split adds no space between two letters.
    SplitsNothing,
    // The split's words, run together, spell something other than the joined word.
    MisspellsTheWord,
    // The quote doesn't hold the joined word whole, spelled exactly as the model spelled it.
    NotInQuote,
    // The word is one of the words to keep whole.
    MustRemainJoined,
    // The bank already splits the word differently.
    ConflictsWithBank,
}

impl JoinedWordBank {
    // Parses the bank from its TOML. Fails on a split that does more than add single spaces between
    // letters of its word, as "they  had" and "her. He" do. Fails too on a word that is both split
    // and kept whole, and on a word kept whole that doesn't start and end with a letter, as
    // "churchgoin'" doesn't.
    pub fn from_toml(toml: &str) -> anyhow::Result<Self> {
        let bank: JoinedWordBank = toml::from_str(toml)?;
        for word in &bank.keep_whole {
            // A proposal's word is trimmed to letters before it's compared with the words kept
            // whole, so a word kept whole with anything else at an end would never match.
            ensure!(
                !word.is_empty() && trim_to_letters(word) == word,
                "the word kept whole {word:?} should start and end with a letter"
            );
        }
        for (joined, split) in &bank.splits {
            // Cleaning leaves a split unchanged when its spaces are single and between letters.
            let is_clean = clean_split(split) == *split;
            ensure!(
                is_clean && split.contains(' ') && split.replace(' ', "") == *joined,
                "the split {split:?} of {joined:?} should be the word with a single space at each \
                 break, each between two letters"
            );
            ensure!(
                !bank.is_kept_whole(joined),
                "{joined:?} should be either split or kept whole, not both"
            );
        }
        Ok(bank)
    }

    // The bank as TOML, opening with `FILE_HEADER`. A comment after each word lists the puzzles
    // among `publishable_quotes` that contain it. Each entry gets a line of its own, so that diffs
    // are clear.
    pub fn to_toml(&self, publishable_quotes: &[(usize, &str)]) -> String {
        let mut toml = format!("{FILE_HEADER}\nkeep_whole = [\n");
        for word in &self.keep_whole {
            let value = word.to_toml_value();
            // A word to keep whole blocks a split in any case, so its comment matches any case too.
            let pattern = whole_word_pattern(&word.to_lowercase());
            let puzzles = list_puzzles_matching(publishable_quotes, |quote| {
                pattern.is_match(&quote.to_lowercase())
            });
            toml.push_str(&format!("    {value},  # {puzzles}\n"));
        }
        toml.push_str("]\n\n[splits]\n");
        for (joined, split) in &self.splits {
            let key = joined.to_toml_key();
            let value = split.to_toml_value();
            let pattern = whole_word_pattern(joined);
            let puzzles =
                list_puzzles_matching(publishable_quotes, |quote| pattern.is_match(quote));
            toml.push_str(&format!("{key} = {value}  # {puzzles}\n"));
        }
        toml
    }

    // Reads the bank from `path`.
    pub async fn read(path: &Path) -> anyhow::Result<Self> {
        let toml = read_path_to_string(path).await?;
        Self::from_toml(&toml).with_context(|| format!("Cannot parse {path:?}"))
    }

    // Writes the bank to `path`.
    pub async fn write(&self, path: &Path, publishable_quotes: &[(usize, &str)]) -> io::Result<()> {
        write_path(path, self.to_toml(publishable_quotes).as_bytes()).await
    }

    // Adds a model's proposed split of a word in `quote` to the bank. Models sometimes include
    // punctuation at either end of the word, or put a space beside punctuation, so the proposal is
    // cleaned first: "theyhad," split as "they had," becomes "theyhad" split as "they had".
    //
    // The cleaned proposal enters the bank only if its split adds a space, and its words, run
    // together, spell the word exactly. The quote must also hold the word whole, spelled exactly as
    // the model spelled it. A model that changes a letter's case or straightens a curly apostrophe
    // therefore has its proposal left out, but another model usually proposes the same split as the
    // quote spells it.
    //
    // A proposal that matches a split the bank already holds is accepted, and the bank stays as it
    // was.
    pub fn add_proposal(
        &mut self,
        quote: &str,
        proposal: &JoinedWord,
    ) -> Result<(), ProposalRejection> {
        let joined = trim_to_letters(&proposal.joined);
        let split = clean_split(&proposal.split);
        if !split.contains(' ') {
            return Err(ProposalRejection::SplitsNothing);
        }
        if split.replace(' ', "") != joined {
            return Err(ProposalRejection::MisspellsTheWord);
        }
        if !whole_word_pattern(joined).is_match(quote) {
            return Err(ProposalRejection::NotInQuote);
        }
        if self.is_kept_whole(joined) {
            return Err(ProposalRejection::MustRemainJoined);
        }
        match self.splits.entry(joined.to_string()) {
            Entry::Vacant(vacant) => {
                vacant.insert(split);
                Ok(())
            }
            Entry::Occupied(occupied) if *occupied.get() == split => Ok(()),
            Entry::Occupied(_) => Err(ProposalRejection::ConflictsWithBank),
        }
    }

    // Splits each of the bank's joined words wherever `quote` holds it whole: "theyhad left"
    // becomes "they had left".
    pub fn split_joined_words(&self, quote: &str) -> String {
        let mut quote = quote.to_string();
        for (joined, split) in &self.splits {
            quote = whole_word_pattern(joined)
                .replace_all(&quote, NoExpand(split))
                .into_owned();
        }
        quote
    }

    // Whether `word` is one of the words that must be kept whole rather than split, ignoring case.
    fn is_kept_whole(&self, word: &str) -> bool {
        let lowercase_word = word.to_lowercase();
        self.keep_whole
            .iter()
            .any(|kept_word| kept_word.to_lowercase() == lowercase_word)
    }
}

// Strips everything but letters from both ends of `text`: "theyhad," becomes "theyhad". Punctuation
// inside the text stays, as in "That’swhat": an entry replaces its word in the quote with its
// split, so splitting that word as "Thats what" would delete the apostrophe from the puzzle's text.
fn trim_to_letters(text: &str) -> &str {
    text.trim_matches(|character: char| !character.is_alphabetic())
}

// Cleans a split into the form the bank stores. Strips everything but letters from its two ends,
// turns each run of whitespace into a single space, and drops each space that isn't between two
// letters. "they  had," becomes "they had", and "get a… I" becomes "get a…I": the ellipsis sits
// inside the split, so trimming the ends leaves it, but the space after it goes.
fn clean_split(split: &str) -> String {
    let mut cleaned = String::new();
    for word in trim_to_letters(split).split_whitespace() {
        let is_between_letters = cleaned.chars().next_back().is_some_and(char::is_alphabetic)
            && word.starts_with(char::is_alphabetic);
        if is_between_letters {
            cleaned.push(' ');
        }
        cleaned.push_str(word);
    }
    cleaned
}

// Lists the numbers of the puzzles whose quotes `matches` accepts, for a word's comment in the
// bank's file. Says `NO_PUBLISHABLE_PUZZLE` when it accepts none.
fn list_puzzles_matching(quotes: &[(usize, &str)], matches: impl Fn(&str) -> bool) -> String {
    let puzzle_numbers: Vec<String> = quotes
        .iter()
        .filter(|(_, quote)| matches(quote))
        .map(|(puzzle_number, _)| puzzle_number.to_string())
        .collect();
    if puzzle_numbers.is_empty() {
        NO_PUBLISHABLE_PUZZLE.to_string()
    } else {
        puzzle_numbers.join(", ")
    }
}

// Matches `word` wherever it stands whole, with no letter, digit, or underscore right before or
// after it: "theyhad" matches in "theyhad left" but not in "theyhadn't".
fn whole_word_pattern(word: &str) -> Regex {
    Regex::new(&format!(r"\b{}\b", regex::escape(word)))
        .expect("an escaped word should make a valid pattern")
}

// Asks `model` for the joined words in each quote, given as stage 0 writes it, and adds each split
// the model proposes to the bank. A message names each proposal the bank refuses, with the puzzle
// number that comes with its quote.
pub async fn add_model_proposals(
    bank: &mut JoinedWordBank,
    quotes: &[(usize, &str)],
    model: &str,
    client: &dyn ChatClient,
) -> anyhow::Result<()> {
    for &(puzzle_number, quote) in quotes {
        let proposals = JoinedWordsRequest {
            quote: quote.to_string(),
        }
        .build(model)?
        .send(client)
        .await?
        .joined_words;
        for proposal in &proposals {
            if let Err(rejection) = bank.add_proposal(quote, proposal) {
                let JoinedWord { joined, split } = proposal;
                eprintln!(
                    "puzzle {puzzle_number}: left out {joined:?} as {split:?}, for {rejection:?}"
                );
            }
        }
    }
    Ok(())
}

// Adds `model`'s proposals to the bank's file. The model reads every publishable quote, meaning
// every quote that `QuoteCatalog::start_puzzle` accepts, or only those among `puzzle_numbers` when
// it's given. The model sees each quote as stage 0 writes it. A message names each puzzle given
// that the model skips, since its quote can't make a puzzle.
//
// Nothing is written until every quote is done. The reply cache keeps the model's replies, though,
// so a rerun after an interruption repeats little work.
pub async fn fill_joined_word_bank(
    model: &str,
    puzzle_numbers: Option<&BTreeSet<usize>>,
    client: &dyn ChatClient,
) -> anyhow::Result<()> {
    let path = joined_word_bank_path();
    let mut bank = JoinedWordBank::read(&path).await?;
    let quote_catalog = QuoteCatalog::load().await?;
    let stage0_quotes: Vec<(usize, String)> = quote_catalog
        .puzzle_numbers()
        .filter_map(|puzzle_number| {
            let puzzle = quote_catalog.start_puzzle(puzzle_number).ok()?;
            Some((puzzle_number, puzzle.quote))
        })
        .collect();
    let publishable_quotes: Vec<(usize, &str)> = stage0_quotes
        .iter()
        .map(|(puzzle_number, quote)| (*puzzle_number, quote.as_str()))
        .collect();
    let chosen_quotes: Vec<(usize, &str)> = publishable_quotes
        .iter()
        .copied()
        .filter(|(puzzle_number, _)| {
            puzzle_numbers.is_none_or(|puzzle_numbers| puzzle_numbers.contains(puzzle_number))
        })
        .collect();
    if let Some(puzzle_numbers) = puzzle_numbers {
        let chosen_numbers: BTreeSet<usize> = chosen_quotes
            .iter()
            .map(|&(puzzle_number, _)| puzzle_number)
            .collect();
        for &puzzle_number in puzzle_numbers.difference(&chosen_numbers) {
            let reason = quote_catalog
                .start_puzzle(puzzle_number)
                .expect_err("a quote that isn't publishable should fail to start a puzzle");
            eprintln!("puzzle {puzzle_number}: skipped, since {reason}");
        }
    }
    add_model_proposals(&mut bank, &chosen_quotes, model, client).await?;
    bank.write(&path, &publishable_quotes).await?;
    let split_count = bank.splits.len();
    println!(
        "the bank splits {split_count} joined words; check each new split before committing {path:?}"
    );
    Ok(())
}
