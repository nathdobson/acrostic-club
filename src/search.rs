use std::collections::HashMap;
use std::default::default;
use std::fmt::Write;
use std::io;
use std::mem::swap;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use crate::alloc::{restore_vec, MmapAllocator};
use crate::dict::FlatWord;
use crate::flat_trie_table::FlatTrieTable;
use crate::model::{Model, Word};
// use crate::trie::Trie;
use crate::{author_title_letters, quote_letters, Letter, LetterMap, LetterSet};

pub struct Search {
    table: FlatTrieTable,
    quote: LetterSet,
    source: Vec<Letter>,
}

impl Search {
    pub async fn new(quote: LetterSet, source: Vec<Letter>) -> io::Result<Self> {
        Ok(Search {
            table: FlatTrieTable::new().await?,
            quote,
            source,
        })
    }
    fn start(&self) -> Solution {
        let mut remainder = self.quote;
        let words: Vec<LetterSet> = self
            .source
            .iter()
            .map(|first| [first].into_iter().cloned().collect())
            .collect();
        for word in words.iter() {
            remainder = remainder - *word;
        }
        Solution { words, remainder }
    }
    fn reset(&self, solution: &mut Solution, index: usize) {
        solution.set_word(index, [self.source[index]].into_iter().collect());
    }
    fn randomize(&self, solution: &mut Solution) {
        for i in 0..thread_rng().gen_range(1..3) {
            self.reset(solution, thread_rng().gen_range(0..self.source.len()));
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
    pub fn anneal(&self, sol: &mut Solution) -> bool {
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
    pub fn solve(&self) -> Option<Solution> {
        for i in 0..100 {
            let mut solution = self.start();
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
        self.remainder = self.remainder - word;
    }
    pub fn words(&self) -> &[LetterSet] { &self.words }
    pub fn is_done(&self) -> bool {
        self.remainder.count() == 0 && self.words.iter().all(|x| x.count() > 1)
    }
}
