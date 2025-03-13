use futures::{stream, SinkExt, StreamExt};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::io::ErrorKind;
use std::mem::swap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Instant;
use std::{io, iter};

use acrostic_core::letter::{Letter, LetterMap, LetterSet};
use itertools::{max, Itertools};
use ordered_float::{NotNan, OrderedFloat};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use safe_once_map::sync::OnceLockMap;

// use crate::trie::Trie;
use crate::dict::FlatWord;
use crate::model::{Model, Word};
use crate::puzzle::{Clue, Puzzle};
use crate::trie_table::{FlatTrieTable, FLAT_TRIE_TABLE};
use crate::util::lazy_async::CloneError;

pub struct Search {
    table: &'static FlatTrieTable,
    cache: OnceLockMap<(Letter, Letter, LetterSet, usize), Vec<(LetterSet, LetterSet)>>,
    access: AtomicUsize,
    quote: LetterSet,
    source: Vec<Letter>,
}

impl Search {
    pub async fn new(quote: LetterSet, source: Vec<Letter>) -> anyhow::Result<Self> {
        Ok(Search {
            table: FLAT_TRIE_TABLE.get().await.clone_error_static()?,
            cache: Default::default(),
            access: AtomicUsize::new(0),
            quote,
            source,
        })
    }
    fn start(&self, seed: u64) -> Option<Solution> {
        let rng = XorShiftRng::seed_from_u64(seed);
        let mut remainder = self.quote;
        let words: Vec<LetterSet> = self
            .source
            .iter()
            .map(|first| [first].into_iter().cloned().collect())
            .collect();
        for word in words.iter() {
            if !word.is_subset(remainder) {
                return None;
            }
            remainder = remainder - *word;
        }
        Some(Solution {
            words,
            remainder,
            rng,
        })
    }
    #[inline(never)]
    fn randomize1(&self, solution: &mut Solution, index: usize) {
        let old = solution.words[index];
        if old.count() > 4 {
            solution.set_word(index, LetterSet::new());
            let mut found = vec![];
            self.table.unary[self.source[index]].search_largest_subset(
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
        self.table.unary[self.source[index]].search_smallest_subset(
            solution.remainder,
            min_len + 1,
            &mut found,
        );
        if found.is_empty() {
            solution.set_word(index, old);
            return false;
        } else {
            found.sort_by_cached_key(|x| {
                NotNan::new(-(x.scrabble_score() as f64 / x.count() as f64)).unwrap()
            });
            let selected = iter::repeat(())
                .take_while(|()| solution.rng.gen_bool(0.5))
                .count()
                .clamp(0, found.len() - 1);
            let selected = found[selected];
            // let selected = *found.choose(&mut solution.rng).unwrap();
            solution.set_word(index, selected);
            return true;
        }
    }
    #[inline(never)]
    fn search_smallest_subset(
        &self,
        l1: Letter,
        l2: Letter,
        key: LetterSet,
        min: usize,
    ) -> &[(LetterSet, LetterSet)] {
        self.access.fetch_add(1, Relaxed);
        &*self.cache[&(l1, l2, key, min)].get_or_init(|| {
            let mut found = vec![];
            self.table
                .binary
                .get(&(l1, l2))
                .unwrap()
                .search_smallest_subset(key, min, &mut found);
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
        let flipped = ls[0] > ls[1];
        if flipped {
            ls.swap(0, 1);
        }
        // let trie = &self.table.binary.get(&(ls[0], ls[1])).unwrap();
        let start = Instant::now();
        let found = self.search_smallest_subset(
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
        let elapsed = start.elapsed();
        if elapsed.as_secs_f64() > 200e-6 {
            // println!("{:?} {:?} {:?} {:?}", ls, solution.remainder, old1.count() + old2.count() + 1, start.elapsed());
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
        for i in 0..10 {
            self.optimize(sol);
            if sol.is_done() {
                // println!("{}", self.format(sol));
                return true;
            }
            self.randomize(sol);
        }
        false
    }
    pub fn solve(&self, seed: u64) -> Option<Solution> {
        let mut solution = self.start(seed)?;
        if self.anneal(&mut solution) {
            return Some(solution);
        } else {
            // println!("failed: {} {} {}", self.format(&solution), self.access.load(Relaxed), self.cache.len());
        }
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
                    write!(&mut result, "{} ", w2.word).unwrap();
                    continue 'main;
                }
            }
            write!(&mut result, "{:?}? ", word).unwrap();
        }
        write!(&mut result, "[{:?}]", sol.remainder).unwrap();
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
    let quote: LetterSet = puzzle
        .quote_letters
        .as_ref()
        .unwrap()
        .bytes()
        .flat_map(|x| Letter::new(x))
        .collect();
    let source: Vec<_> = puzzle
        .source_letters
        .as_ref()
        .unwrap()
        .bytes()
        .flat_map(|x| Letter::new(x))
        .collect();
    // println!("{:?}", source);
    let search = Arc::new(Search::new(quote, source).await?);
    let sol = stream::iter(0..1000)
        .map(|seed| {
            let search = search.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    stream::iter(search.solve((pindex as u64) * 1000 + seed))
                })
                .await
                .unwrap()
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
            let mut result: Vec<_> = puzzle
                .quote_letters
                .as_ref()
                .unwrap()
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
            clue: None,
            answer: w.word.to_string(),
            answer_letters: w.letter_vec.iter().join(""),
            indices: w
                .letter_vec
                .iter()
                .map(|l| positions[*l].pop().unwrap())
                .collect(),
        })
        .collect();
    let mut clues2 = LetterMap::<Vec<Clue>>::new();
    for clue in clues {
        clues2[Letter::new(clue.answer_letters.bytes().next().expect("first letter"))
            .expect("ascii")]
        .push(clue);
    }
    // println!("{:?}", clues2);
    // println!("{:?}", puzzle.source_letters);
    let clues3 = puzzle
        .source_letters
        .as_ref()
        .unwrap()
        .bytes()
        .map(|x| {
            let x = Letter::new(x).unwrap();
            clues2[x].pop().unwrap_or_else(|| panic!("Missing {:?}", x))
        })
        .collect();
    puzzle.clues = Some(clues3);
    puzzle.write(pindex, "stage2.json").await?;
    Ok(())
}

#[tokio::test]
async fn test_search() -> anyhow::Result<()> {
    let firsts = (Letter::new(b'e').unwrap(), Letter::new(b'i').unwrap());
    let word = LetterSet::from_str("AEEEEEEEEEEEGIIKKKKLLLNNNOPPTTTWWWW");
    FLAT_TRIE_TABLE.get().await.clone_error()?;
    let start = Instant::now();
    for i in 0..10000 {
        let mut found = vec![];
        FLAT_TRIE_TABLE
            .get()
            .await
            .clone_error()?
            .binary
            .get(&firsts)
            .unwrap()
            .search_smallest_subset(word, 16, &mut found);
    }
    println!("{:?}", start.elapsed());
    Ok(())
}
