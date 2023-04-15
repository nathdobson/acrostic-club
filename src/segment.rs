use any_ascii::any_ascii;
use itertools::EitherOrBoth;
use unicode_segmentation::UnicodeSegmentation;

use crate::Letter;

pub fn segment(s: &str) -> Vec<EitherOrBoth<Letter, String>> {
    let mut result = vec![];
    for grapheme in s.graphemes(true) {
        let mut letters = vec![];
        let ascii = any_ascii(grapheme);
        for c in ascii.bytes() {
            if let Ok(l) = Letter::new(c) {
                letters.push(l);
            }
        }
        if letters.len() == 0 {
            result.push(EitherOrBoth::Right(grapheme.to_string()));
        } else {
            for (index, letter) in letters.iter().enumerate() {
                if index == 0 {
                    result.push(EitherOrBoth::Both(*letter, grapheme.to_string()));
                } else {
                    result.push(EitherOrBoth::Left(*letter))
                }
            }
        }
    }
    result
}
