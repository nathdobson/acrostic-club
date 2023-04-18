use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::fs;
use std::io::{BufRead, Cursor, Read};
use std::rc::Rc;

use ordered_float::NotNan;
use acrostic_core::letter::{Letter, LetterSet};
use any_ascii::any_ascii;

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Clone, Copy, Debug)]
pub enum Part {
    Noun,
    Verb,
    Adj,
    Num,
    Adv,
    Propn,
    Intj,
    X,
    Sym,
}

pub struct Word {
    pub word: String,
    pub letter_vec: Vec<Letter>,
    pub letters: LetterSet,
    pub part: Part,
    pub weights: Vec<f32>,
}

pub struct Model {
    pub words: Vec<Rc<Word>>,
    // pub letter_map: HashMap<LetterMap<u8>, Vec<Rc<Word>>>,
    // pub by_candidates: HashMap<LetterMap<bool>, Vec<LetterMap<u8>>>,
}

impl Word {
    fn length(&self) -> f32 { self.weights.iter().map(|x| *x * *x).sum::<f32>().sqrt() }
    fn distance(&self, other: &Word) -> f32 {
        self.weights
            .iter()
            .zip(other.weights.iter())
            .map(|(x, y)| (*x * *y))
            .sum::<f32>()
            / (self.length() * other.length())
    }
}

impl Model {
    pub fn get() -> Model { Model::new(&fs::read("data/223/model.bin").unwrap()) }
    pub fn new(model: &[u8]) -> Self {
        let mut cursor = Cursor::new(model);
        let mut line = String::new();
        cursor.read_line(&mut line).unwrap();
        let mut split = line.split_ascii_whitespace();
        let words: usize = split.next().unwrap().parse().unwrap();
        let dims: usize = split.next().unwrap().parse().unwrap();
        let word_vec = (0..words)
            .map(|_| {
                let mut header = vec![];
                cursor.read_until(b' ', &mut header).unwrap();
                header.pop();
                let [word, part]: [&[u8]; 2] = header
                    .split(|x| x == &b'_')
                    .collect::<Vec<&[u8]>>()
                    .try_into()
                    .unwrap();
                let word = std::str::from_utf8(word).unwrap().to_string();
                let part = match part {
                    b"NOUN" => Part::Noun,
                    b"NUM" => Part::Num,
                    b"ADV" => Part::Adv,
                    b"VERB" => Part::Verb,
                    b"ADJ" => Part::Adj,
                    b"PROPN" => Part::Propn,
                    b"INTJ" => Part::Intj,
                    b"X" => Part::X,
                    b"SYM" => Part::Sym,
                    _ => panic!("{:?}", std::str::from_utf8(part)),
                };
                let weights = (0..dims)
                    .map(|_| {
                        let mut b = [0u8; 4];
                        cursor.read_exact(&mut b).unwrap();
                        f32::from_le_bytes(b)
                    })
                    .collect();
                let mut letter_vec = vec![];
                let mut letters = LetterSet::new();
                for c in any_ascii(&word).chars() {
                    if let Ok(letter) = Letter::new(c.try_into().unwrap()) {
                        letter_vec.push(letter);
                        letters[letter] += 1;
                    }
                }
                Rc::new(Word {
                    word,
                    letter_vec,
                    letters,
                    part,
                    weights,
                })
            })
            .collect::<Vec<_>>();
        // let mut letter_map: HashMap<LetterMap<u8>, Vec<Rc<Word>>> = HashMap::new();
        // for word in word_vec.iter() {
        //
        //     letter_map.entry(letters).or_default().push(word.clone());
        // }
        // let mut by_candidates: HashMap<LetterMap<bool>, LetterMap<u8>> = HashMap::new();
        // for (letters, _) in words {
        //     by_candidates
        //         .entry(letters.map(|x| x > 0))
        //         .or_default()
        //         .push(letters);
        // }
        Model {
            words: word_vec,
            // letter_map,
        }
    }
    pub fn sorted(&self, word: &str, part: Part) -> Vec<LetterSet> {
        let center = self
            .words
            .iter()
            .find(|x| x.word == word && x.part == part)
            .unwrap();
        let mut words = self
            .words
            .iter()
            .map(|x| (x, -x.distance(center)))
            .collect::<Vec<_>>();
        words.sort_by_key(|(i, x)| NotNan::new(*x).unwrap());
        let words = words.iter().map(|(i, x)| (*i).clone()).collect::<Vec<_>>();
        let mut maps = vec![];
        let mut visited = HashSet::new();
        for x in words {
            if visited.insert(x.letters.clone()) {
                maps.push(x.letters);
            }
        }
        maps
    }
}

impl Debug for Word {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{:?}", self.word, self.part)
    }
}

#[test]
fn read_model() { Model::get(); }

// #[test]
// fn test_sort() {
//     let model = Model::get();
//     println!("{:?}", &model.sorted("berkeley", Part::Noun)[0..100]);
//     println!("{:?}", &model.sorted("berkeley", Part::Propn)[0..100]);
// }
