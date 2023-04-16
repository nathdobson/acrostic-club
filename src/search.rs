use std::collections::HashMap;
use std::default::default;
use std::fmt::Write;
use std::io;
use std::mem::swap;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng, SeedableRng};

use crate::alloc::{restore_vec, MmapAllocator};
use crate::dict::FlatWord;
use crate::trie_table::FlatTrieTable;
use crate::model::{Model, Word};
// use crate::trie::Trie;
use crate::{author_title_letters, quote_letters, Letter, LetterMap, LetterSet};
use crate::puzzle::{Clue, Puzzle};
use itertools::Itertools;
use rand::rngs::StdRng;
use rand_xorshift::XorShiftRng;

pub struct Search {
    table: FlatTrieTable,
    quote: LetterSet,
    source: Vec<Letter>,
    rng: XorShiftRng,
}

impl Search {
    pub async fn new(quote: LetterSet, source: Vec<Letter>, seed: u64) -> io::Result<Self> {
        Ok(Search {
            table: FlatTrieTable::new().await?,
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
            if !word.is_subset(remainder){
                return None;
            }
            remainder = remainder - *word;
        }
        Some(Solution { words, remainder })
    }
    fn reset(&self, solution: &mut Solution, index: usize) {
        solution.set_word(index, [self.source[index]].into_iter().collect());
    }
    fn randomize(&mut self, solution: &mut Solution) {
        for _ in 0..self.rng.gen_range(1..3) {
            let index = self.rng.gen_range(0..self.source.len());
            self.reset(solution, index);
        }
    }
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
    fn optimize2(&self, solution: &mut Solution, i1: usize, i2: usize) -> bool {
        let old1 = solution.words[i1];
        let old2 = solution.words[i2];
        solution.set_word(i1, LetterSet::new());
        solution.set_word(i2, LetterSet::new());
        let mut ls = [self.source[i1], self.source[i2]];
        let flipped = ls[0] > ls[1];
        if flipped {
            ls.swap(0, 1);
        }
        let trie = &self.table.binary.get(&(ls[0], ls[1])).unwrap();
        if let Some(found) =
        trie.search_smallest_subset(solution.remainder, old1.count() + old2.count() + 1)
        {
            if flipped {
                solution.set_word(i1, found.1);
                solution.set_word(i2, found.0);
            } else {
                solution.set_word(i1, found.0);
                solution.set_word(i2, found.1);
            }
            return true;
        }
        solution.set_word(i1, old1);
        solution.set_word(i2, old2);
        return false;
    }
    fn optimize(&self, solution: &mut Solution) {
        loop {
            let mut progress = false;
            for i in 0..solution.words.len() {
                progress |= self.optimize1(solution, i);
            }
            if !progress {
                break;
            }
        }
        loop {
            let mut progress = false;
            for i1 in 0..solution.words.len() {
                for i2 in 0..solution.words.len() {
                    if i1 < i2 {
                        progress |= self.optimize2(solution, i1, i2);
                    }
                }
            }
            if !progress {
                break;
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
        for i in 0..100 {
            let mut solution = self.start()?;
            if self.anneal(&mut solution) {
                return Some(solution);
            } else {
                println!("failed: {}", self.format(&solution));
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
        self.remainder.count() == 0 && self.words.iter().all(|x| x.count() > 1)
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
    let sol = search.solve().ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "timed out"))?;
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