#![allow(unused_imports)]
#![allow(unused_variables)]
#![deny(unused_must_use)]
#![allow(unused_mut)]

use std::collections::HashMap;
use std::io;

use acrostic::letter::{Letter, LetterMap, LetterSet};
use acrostic::puzzle::{Clue, Puzzle};
use acrostic::search::Search;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
// use acrostic::puzzle::{Cell, Puzzle};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut puzzle = Puzzle::read("stage1.json")?;
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
    let search = Search::new(quote, source);
    let sol = search.solve().expect("no solution found");
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
    puzzle.write("stage2.json")?;
    Ok(())
}
