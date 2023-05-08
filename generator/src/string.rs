use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use unicode_segmentation::UnicodeSegmentation;
use acrostic_core::letter::{Letter, LetterSet};
use any_ascii::any_ascii;
use itertools::Itertools;

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Clone)]
pub struct LetterString(Vec<Letter>);

#[derive(Clone)]
pub struct Grapheme {
    string: String,
    letters: Vec<Letter>,
}

#[derive(Clone)]
pub struct GraphemeString(Vec<Grapheme>);

impl GraphemeString {
    pub fn from_str(x: &str) -> Self {
        let mut result = vec![];
        for grapheme in x.graphemes(true) {
            let mut letters = vec![];
            let ascii = any_ascii(grapheme);
            for c in ascii.bytes() {
                if let Ok(l) = Letter::new(c) {
                    letters.push(l);
                }
            }
            result.push(Grapheme {
                string: grapheme.to_string(),
                letters,
            });
        }
        GraphemeString(result)
    }
    pub fn graphemes(&self) -> impl Iterator<Item=&Grapheme> {
        self.0.iter()
    }
    pub fn letters<'a>(&'a self) -> impl 'a + Iterator<Item=Letter> {
        self.0.iter().flat_map(|x| x.letters.iter().cloned())
    }
    pub fn stems(&self) -> impl Iterator<Item=&[Grapheme]> {
        self.0.split(|x| if x.letters.is_empty() {
            match &*x.string {
                "-" => true,
                "'" | "_" | "ʼ" | "ʽ" | "ʻ" | "ʿ" => false,
                _ => {
                    true
                }
            }
        } else { false })
    }
}

impl LetterString {
    pub fn from_graphemes(g: &GraphemeString) -> Self {
        LetterString(g.letters().collect())
    }
    pub fn from_str(x: &str) -> Self {
        LetterString::from_graphemes(&GraphemeString::from_str(x))
    }
}

impl AsRef<[Letter]> for LetterString {
    fn as_ref(&self) -> &[Letter] {
        &*self.0
    }
}

impl Deref for LetterString {
    type Target = [Letter];
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Borrow<[Letter]> for LetterString {
    fn borrow(&self) -> &[Letter] {
        &*self.0
    }
}

impl Debug for GraphemeString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in self.graphemes() {
            write!(f, "{:?}", x)?;
        }
        Ok(())
    }
}

impl Debug for Grapheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.string.is_ascii() && self.letters.len() == 1 {
            write!(f, "{}", self.letters[0])?;
        } else if self.letters.is_empty() {
            write!(f, "{}", self.string)?;
        } else {
            write!(f, "[{}/{}]", self.letters.iter().join(""), self.string)?;
        }
        Ok(())
    }
}

impl Debug for LetterString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in &self.0 {
            write!(f, "{}", x)?;
        }
        Ok(())
    }
}

impl FromIterator<Letter> for LetterString {
    fn from_iter<T: IntoIterator<Item=Letter>>(iter: T) -> Self {
        LetterString(iter.into_iter().collect())
    }
}

impl FromIterator<Grapheme> for GraphemeString {
    fn from_iter<T: IntoIterator<Item=Grapheme>>(iter: T) -> Self {
        GraphemeString(iter.into_iter().collect())
    }
}

#[test]
fn test_string() {
    assert_eq!(format!("{:?}", GraphemeString::from_str("*straßé*")), "*STRA[SS/ß][E/é]*");
}