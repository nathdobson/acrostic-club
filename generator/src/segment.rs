use std::fmt::Write;
use std::io;

use itertools::EitherOrBoth;
use unicode_segmentation::UnicodeSegmentation;
use acrostic_core::letter::Letter;
use any_ascii::any_ascii;

use crate::puzzle::Puzzle;

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


fn get_letters(input: &str) -> String {
    let mut cells = String::new();
    for x in segment(&input) {
        if let Some(letter) = x.as_ref().left() {
            write!(&mut cells, "{}", letter).unwrap();
        } else if let Some(content) = x.right() {
            if content.chars().all(|x| x.is_numeric()) {
                // write!(&mut cells, "{}",content).unwrap();
                continue;
            }
            match &*content {
                " " => {
                    write!(&mut cells, "{}", content).unwrap();
                }
                "." | "," | ";" | "'" | "\"" | "!" | "?" | "‘" | "’" | ":"
                | "&" | "*" | "(" | ")" | "”" | "“" | "…" | "\n" | "\u{a0}"
                | "$" | "~" | "\t" | "_" | "/" | "´" | "[" | "]" | "#" => {}
                "-" | "—" | "–" => {
                    write!(&mut cells, "-").unwrap();
                }
                x => { panic!("{:?}", x); }
            }
        }
    }
    cells
}

pub fn get_alpha(x: &str) -> Vec<Letter> {
    segment(x).into_iter().flat_map(|x| x.left()).collect()
}

pub async fn add_letters(pindex: usize) -> io::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage0.json").await?;
    puzzle
        .quote_letters
        .get_or_insert_with(|| get_letters(&puzzle.quote));
    puzzle.source_letters.get_or_insert_with(|| {
        get_letters(&puzzle.source)
            .chars()
            .filter(|x| x.is_ascii_alphabetic())
            .collect()
    });
    puzzle.write(pindex, "stage1.json").await?;
    Ok(())
}