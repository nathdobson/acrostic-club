#![allow(unused_imports)]
#![allow(unused_variables)]

use std::io::Error;
use std::path::Path;
use std::{env, fs, io};

use acrostic::alloc::save_vec;
// use acrostic::dict::{Dict, DictWord};
use acrostic::flat_dict::FlatWord;
use acrostic::letter::{Letter, LetterSet};
use acrostic::PACKAGE_PATH;
use any_ascii::any_ascii;
use arrayvec::ArrayVec;

fn main() -> io::Result<()> {
    let contents = fs::read_to_string(
        PACKAGE_PATH.join("wikipedia-word-frequency/results/enwiki-2022-08-29.txt"),
    )?;

    let mut words = vec![];
    for line in contents.split("\n") {
        if line.is_empty() {
            continue;
        }
        let (word, freq) = line.split_once(" ").unwrap();
        let freq: usize = freq.parse().unwrap();
        let mut letter_vec = ArrayVec::new();
        let mut letters = LetterSet::new();
        for c in any_ascii(&word).chars() {
            if let Ok(letter) = Letter::new(c.try_into().unwrap()) {
                letter_vec.push(letter);
                letters[letter] += 1;
            }
        }
        words.push(FlatWord {
            word: (*word.to_string()).try_into().unwrap(),
            letter_vec,
            letters,
            frequency: freq as u64,
        })
    }

    save_vec(&PACKAGE_PATH.join("index/dict.dat"), &words);
    Ok(())
}
