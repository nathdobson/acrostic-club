use anyhow::Context;
use futures::{StreamExt, stream};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::sync::Arc;
#[cfg(feature = "search-diagnostics")]
use std::sync::atomic::AtomicUsize;
#[cfg(feature = "search-diagnostics")]
use std::sync::atomic::Ordering::Relaxed;
#[cfg(feature = "search-diagnostics")]
use std::time::Instant;
use std::{io, iter};

use acrostic_core::letter::{Letter, LetterMap, LetterSet};
use itertools::Itertools;
use ordered_float::NotNan;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use safe_once_map::sync::OnceLockMap;

use crate::dict::FlatWord;
use crate::puzzle::{Clue, Puzzle};
use crate::trie_table::{FLAT_TRIE_TABLE, FlatTrieTable};
use crate::util::lazy_async::CloneError;

pub struct Search {
    table: &'static FlatTrieTable,
    cache: OnceLockMap<(Letter, Letter, LetterSet, usize), Vec<(LetterSet, LetterSet)>>,
    // Counts calls to `search_word_pairs`, for the report on each failed seed.
    #[cfg(feature = "search-diagnostics")]
    access: AtomicUsize,
    quote: LetterSet,
    source: Vec<Letter>,
}

impl Search {
    // Panics when the source uses some letter more often than the quote does, since no answers
    // could spell it. Stage 0 rejects such quotes.
    pub async fn new(quote: LetterSet, source: Vec<Letter>) -> anyhow::Result<Self> {
        let source_letter_counts: LetterSet = source.iter().copied().collect();
        assert!(
            source_letter_counts.is_subset(quote),
            "stage 0 should have rejected a quote that lacks letters its source needs"
        );
        Ok(Search {
            table: FLAT_TRIE_TABLE.get().await.clone_error_static()?,
            cache: Default::default(),
            #[cfg(feature = "search-diagnostics")]
            access: AtomicUsize::new(0),
            quote,
            source,
        })
    }
    fn start(&self, seed: u64) -> Solution {
        let rng = XorShiftRng::seed_from_u64(seed);
        let mut remainder = self.quote;
        let words: Vec<LetterSet> = self
            .source
            .iter()
            .map(|first| [first].into_iter().cloned().collect())
            .collect();
        // `Search::new` checked that the quote holds every source letter.
        for word in words.iter() {
            remainder = remainder - *word;
        }
        Solution {
            words,
            remainder,
            rng,
        }
    }
    #[inline(never)]
    fn randomize1(&self, solution: &mut Solution, index: usize) {
        let old = solution.words[index];
        if old.count() > 4 {
            solution.set_word(index, LetterSet::new());
            let mut found = vec![];
            self.table.tries[self.source[index]].search_largest_subset(
                solution.remainder,
                old.count() - 1,
                &mut found,
            );
            if let Some(found) = found.choose(&mut solution.rng) {
                solution.set_word(index, *found);
            } else {
                solution.set_word(index, old);
            }
        }
    }
    #[inline(never)]
    fn randomize(&self, solution: &mut Solution) {
        let limit = solution.rng.gen_range(1..3);
        for _ in 0..limit {
            let index = solution.rng.gen_range(0..self.source.len());
            self.randomize1(solution, index);
        }
    }
    #[inline(never)]
    fn optimize1(&self, solution: &mut Solution, index: usize) -> bool {
        let old = solution.words[index];
        solution.set_word(index, LetterSet::new());
        let min_len = old.count();
        let mut found = vec![];
        self.table.tries[self.source[index]].search_subsets_with_min_length(
            solution.remainder,
            min_len + 1,
            &mut found,
        );
        if found.is_empty() {
            solution.set_word(index, old);
            return false;
        } else {
            // Rank the words by score per letter, best first, then from shortest to longest. The
            // sort is stable, so words that tie on both keep their order in the trie.
            found.sort_by_cached_key(|x| {
                (
                    NotNan::new(-(x.scrabble_score() as f64 / x.count() as f64))
                        .expect("a word should hold at least one letter"),
                    x.count(),
                )
            });
            let selected = iter::repeat(())
                .take_while(|()| solution.rng.gen_bool(0.5))
                .count()
                .clamp(0, found.len() - 1);
            let selected = found[selected];
            solution.set_word(index, selected);
            return true;
        }
    }
    // Finds the same pairs as `FlatTrieTable::search_word_pairs`, and caches them so that a
    // repeated query costs only a lookup.
    #[inline(never)]
    fn search_word_pairs(
        &self,
        first_letter: Letter,
        second_letter: Letter,
        available_letters: LetterSet,
        min_length: usize,
    ) -> &[(LetterSet, LetterSet)] {
        #[cfg(feature = "search-diagnostics")]
        self.access.fetch_add(1, Relaxed);
        let cache_key = (first_letter, second_letter, available_letters, min_length);
        &*self.cache[&cache_key].get_or_init(|| {
            let mut found = vec![];
            self.table.search_word_pairs(
                first_letter,
                second_letter,
                available_letters,
                min_length,
                &mut found,
            );
            found
        })
    }
    #[inline(never)]
    fn optimize2(&self, solution: &mut Solution, i1: usize, i2: usize, max_len: usize) -> bool {
        let old1 = solution.words[i1];
        let old2 = solution.words[i2];
        solution.set_word(i1, LetterSet::new());
        solution.set_word(i2, LetterSet::new());
        let mut ls = [self.source[i1], self.source[i2]];
        // Put the two letters in alphabetical order, so that the queries for either order share one
        // cache entry.
        let flipped = ls[0] > ls[1];
        if flipped {
            ls.swap(0, 1);
        }
        #[cfg(feature = "search-diagnostics")]
        let start = Instant::now();
        let found = self.search_word_pairs(
            ls[0],
            ls[1],
            solution.remainder,
            old1.count() + old2.count() + 1,
        );
        if let Some(found) = found.choose(&mut solution.rng) {
            if found.0.count() <= max_len || found.1.count() <= max_len {
                if flipped {
                    solution.set_word(i1, found.1);
                    solution.set_word(i2, found.0);
                } else {
                    solution.set_word(i1, found.0);
                    solution.set_word(i2, found.1);
                }
                return true;
            }
        }
        // Reports a pair search slow enough to matter. Only searches that found no swap reach here.
        #[cfg(feature = "search-diagnostics")]
        {
            let elapsed = start.elapsed();
            if elapsed.as_secs_f64() > 200e-6 {
                println!(
                    "Slow pair search for {ls:?} found no swap in {elapsed:?}: remainder {:?}, \
                     minimum length {}",
                    solution.remainder,
                    old1.count() + old2.count() + 1
                );
            }
        }
        solution.set_word(i1, old1);
        solution.set_word(i2, old2);
        return false;
    }
    #[inline(never)]
    fn optimize(&self, solution: &mut Solution) {
        'outer: for max_len in 6.. {
            loop {
                let mut progress = false;
                let mut missed = false;
                let mut indices = (0..solution.words.len()).collect::<Vec<_>>();
                indices.shuffle(&mut solution.rng);
                for i in indices {
                    if solution.words[i].count() < max_len {
                        progress |= self.optimize1(solution, i);
                    } else {
                        missed = true;
                    }
                }
                if !progress {
                    if missed {
                        break;
                    } else {
                        break 'outer;
                    }
                }
            }
            // println!("{:?}", solution);
            loop {
                let mut progress = false;
                for i1 in 0..solution.words.len() {
                    for i2 in 0..solution.words.len() {
                        if i1 < i2 {
                            progress |= self.optimize2(solution, i1, i2, max_len);
                        }
                    }
                }
                if !progress {
                    break;
                }
            }
        }
    }
    pub fn anneal(&self, sol: &mut Solution) -> bool {
        for _ in 0..10 {
            self.optimize(sol);
            if sol.is_done() {
                return true;
            }
            self.randomize(sol);
        }
        false
    }
    pub fn solve(&self, seed: u64) -> Option<Solution> {
        let mut solution = self.start(seed);
        if self.anneal(&mut solution) {
            return Some(solution);
        }
        #[cfg(feature = "search-diagnostics")]
        println!(
            "Seed {seed} failed with {}; {} pair searches so far, {} of them distinct",
            self.format(&solution),
            self.access.load(Relaxed),
            self.cache.len()
        );
        return None;
    }
    pub fn get_words(&self, sol: &Solution) -> Vec<&FlatWord> {
        let mut result = vec![];
        'main: for (i, word) in sol.words.iter().enumerate() {
            for w2 in &*self.table.dict {
                if w2.letters == *word && w2.letter_vec.first() == Some(&self.source[i]) {
                    result.push(w2);
                    continue 'main;
                }
            }
            unreachable!();
        }
        result
    }
    pub fn format(&self, sol: &Solution) -> String {
        let mut result = String::new();
        'main: for (i, word) in sol.words.iter().enumerate() {
            for w2 in &*self.table.dict {
                if w2.letters == *word && w2.letter_vec.first() == Some(&self.source[i]) {
                    result += &format!("{} ", w2.word);
                    continue 'main;
                }
            }
            result += &format!("{word:?}? ");
        }
        result += &format!("[{:?}]", sol.remainder);
        result
    }
}

#[derive(Debug)]
pub struct Solution {
    words: Vec<LetterSet>,
    remainder: LetterSet,
    rng: XorShiftRng,
}

impl Solution {
    pub fn set_word(&mut self, index: usize, word: LetterSet) {
        self.remainder = self.remainder + self.words[index];
        self.words[index] = word;
        assert!(word.is_subset(self.remainder));
        self.remainder = self.remainder - word;
    }
    pub fn words(&self) -> &[LetterSet] {
        &self.words
    }
    pub fn is_done(&self) -> bool {
        self.remainder.count() == 0
            && self.words.iter().all(|x| x.count() > 1)
            && self.words.iter().collect::<HashSet<_>>().len() == self.words.len()
    }
}

pub async fn add_answers(pindex: usize) -> anyhow::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage1.json").await?;
    let quote_letters = puzzle
        .quote_letters
        .as_deref()
        .with_context(|| format!("Puzzle {pindex}'s stage1.json lacks `quote_letters`"))?;
    let source_letters = puzzle
        .source_letters
        .as_deref()
        .with_context(|| format!("Puzzle {pindex}'s stage1.json lacks `source_letters`"))?;
    let quote: LetterSet = quote_letters.bytes().flat_map(|x| Letter::new(x)).collect();
    let source: Vec<_> = source_letters
        .bytes()
        .flat_map(|x| Letter::new(x))
        .collect();
    let search = Arc::new(Search::new(quote, source).await?);
    let sol = stream::iter(0..1000)
        .map(|seed| {
            let search = search.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    stream::iter(search.solve((pindex as u64) * 1000 + seed))
                })
                .await
                .expect("a search task should finish without panicking")
            }
        })
        .buffered(num_cpus::get())
        .flatten()
        .next()
        .await
        .ok_or(io::Error::new(ErrorKind::TimedOut, "timed out"))?;
    let words = search.get_words(&sol);
    let mut rng = XorShiftRng::seed_from_u64(pindex as u64);
    let mut positions: LetterMap<Vec<usize>> = Letter::all()
        .map(|l| {
            let mut result: Vec<_> = quote_letters
                .chars()
                .positions(|l2| l.to_char() == l2)
                .collect();
            result.shuffle(&mut rng);
            result
        })
        .collect();
    let clues: Vec<Clue> = words
        .iter()
        .map(|w| Clue {
            clue_tag: None,
            answer: w.word.to_string(),
            answer_letters: w.letter_vec.iter().join(""),
            indices: w
                .letter_vec
                .iter()
                .map(|l| {
                    positions[*l]
                        .pop()
                        .expect("the answers should use no more of a letter than the quote holds")
                })
                .collect(),
        })
        .collect();
    let mut clues2 = LetterMap::<Vec<Clue>>::new();
    for clue in clues {
        clues2[Letter::new(
            clue.answer_letters
                .bytes()
                .next()
                .expect("an answer should hold at least one letter"),
        )
        .expect("`answer_letters` should hold only letters")]
        .push(clue);
    }
    let clues3 = source_letters
        .bytes()
        .map(|byte| {
            let letter =
                Letter::new(byte).expect("stage 1 should record only letters in `source_letters`");
            clues2[letter].pop().unwrap_or_else(|| {
                panic!("No answer left starts with {letter:?}; unplaced answers: {clues2:?}")
            })
        })
        .collect();
    puzzle.clues = Some(clues3);
    puzzle.write(pindex, "stage2.json").await?;
    Ok(())
}
