use std::collections::{HashMap, HashSet};
use std::default::default;
use std::fmt::Write;
use std::io;
use std::mem::swap;
use std::time::Instant;

use itertools::{Itertools, max};
use rand::{Rng, SeedableRng, thread_rng};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand_xorshift::XorShiftRng;
use acrostic_core::letter::{Letter, LetterMap, LetterSet};

// use crate::trie::Trie;
use crate::dict::FlatWord;
use crate::model::{Model, Word};
use crate::puzzle::{Clue, Puzzle};
use crate::trie_table::{FLAT_TRIE_TABLE, FlatTrieTable};
use crate::util::lazy_async::CloneError;

pub struct Search {
    table: &'static FlatTrieTable,
    cache: HashMap<(Letter, Letter, LetterSet, usize), Option<(LetterSet, LetterSet)>>,
    access: usize,
    quote: LetterSet,
    source: Vec<Letter>,
    rng: XorShiftRng,
}

impl Search {
    pub async fn new(quote: LetterSet, source: Vec<Letter>, seed: u64) -> io::Result<Self> {
        Ok(Search {
            table: FLAT_TRIE_TABLE.get().await.clone_error()?,
            cache: Default::default(),
            access: 0,
            quote,
            source,
            rng: XorShiftRng::seed_from_u64(seed),
        })
    }
    fn start(&self) -> Option<Solution> {
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
        Some(Solution { words, remainder })
    }
    #[inline(never)]
    fn randomize1(&self, solution: &mut Solution, index: usize) {
        let old = solution.words[index];
        if old.count() > 4 {
            solution.set_word(index, LetterSet::new());
            if let Some(found) = self.table.unary[self.source[index]]
                .search_largest_subset(solution.remainder, old.count() - 1)
            {
                solution.set_word(index, *found);
            } else {
                solution.set_word(index, old);
            }
        }
    }
    #[inline(never)]
    fn randomize(&mut self, solution: &mut Solution) {
        for _ in 0..self.rng.gen_range(1..3) {
            let index = self.rng.gen_range(0..self.source.len());
            self.randomize1(solution, index);
        }
    }
    #[inline(never)]
    fn optimize1(&self, solution: &mut Solution, index: usize) -> bool {
        let old = solution.words[index];
        solution.set_word(index, LetterSet::new());
        let min_len = old.count();
        if let Some(found) = self.table.unary[self.source[index]]
            .search_smallest_subset(solution.remainder, min_len + 1)
        {
            solution.set_word(index, *found);
            return true;
        }

        solution.set_word(index, old);
        return false;
    }
    #[inline(never)]
    fn search_smallest_subset(&mut self, l1: Letter, l2: Letter, key: LetterSet, min: usize) -> Option<(LetterSet, LetterSet)> {
        self.access += 1;
        *self.cache.entry((l1, l2, key, min)).or_insert_with(|| {
            self.table.binary.get(&(l1, l2)).unwrap().search_smallest_subset(key, min).cloned()
        })
    }
    #[inline(never)]
    fn optimize2(&mut self, solution: &mut Solution, i1: usize, i2: usize, max_len: usize) -> bool {
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
        if let Some(found) =
        self.search_smallest_subset(ls[0], ls[1], solution.remainder, old1.count() + old2.count() + 1)
        {
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
    fn optimize(&mut self, solution: &mut Solution) {
        'outer: for max_len in 6.. {
            loop {
                let mut progress = false;
                let mut missed = false;
                let mut indices = (0..solution.words.len()).collect::<Vec<_>>();
                indices.shuffle(&mut self.rng);
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
    pub fn anneal(&mut self, sol: &mut Solution) -> bool {
        for i in 0..10 {
            self.optimize(sol);
            if sol.is_done() {
                println!("{}", self.format(sol));
                return true;
            }
            self.randomize(sol);
        }
        false
    }
    pub fn solve(&mut self) -> Option<Solution> {
        for i in 0..1000 {
            let mut solution = self.start()?;
            if self.anneal(&mut solution) {
                return Some(solution);
            } else {
                println!("failed: {} {} {}", self.format(&solution), self.access, self.cache.len());
            }
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
}

impl Solution {
    pub fn set_word(&mut self, index: usize, word: LetterSet) {
        self.remainder = self.remainder + self.words[index];
        self.words[index] = word;
        assert!(word.is_subset(self.remainder));
        self.remainder = self.remainder - word;
    }
    pub fn words(&self) -> &[LetterSet] { &self.words }
    pub fn is_done(&self) -> bool {
        self.remainder.count() == 0
            && self.words.iter().all(|x| x.count() > 1)
            && self.words.iter().collect::<HashSet<_>>().len() == self.words.len()
    }
}

pub async fn add_answers(pindex: usize) -> io::Result<()> {
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
    println!("{:?}", source);
    let mut search = Search::new(quote, source, pindex as u64).await?;
    let sol = search.solve().ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, format!("timed out {}", pindex)))?;
    let words = search.get_words(&sol);
    let mut positions: LetterMap<Vec<usize>> = Letter::all()
        .map(|l| {
            let mut result: Vec<_> = puzzle
                .quote_letters
                .as_ref()
                .unwrap()
                .chars()
                .positions(|l2| l.to_char() == l2)
                .collect();
            result.shuffle(&mut thread_rng());
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
        clues2[Letter::new(clue.answer.bytes().next().unwrap()).unwrap()].push(clue);
    }
    println!("{:?}", clues2);
    println!("{:?}", puzzle.source_letters);
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
async fn test_search() -> io::Result<()> {
    let firsts = (Letter::new(b'e').unwrap(), Letter::new(b'i').unwrap());
    let word = LetterSet::from_str("AEEEEEEEEEEEGIIKKKKLLLNNNOPPTTTWWWW");
    FLAT_TRIE_TABLE.get().await.clone_error()?;
    let start = Instant::now();
    for i in 0..10000 {
        FLAT_TRIE_TABLE.get().await.clone_error()?.binary.get(&firsts).unwrap().search_smallest_subset(word, 16);
    }
    println!("{:?}", start.elapsed());
    Ok(())
}