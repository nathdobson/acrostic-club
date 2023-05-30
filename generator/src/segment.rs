use std::fmt::Write;
use std::io;

use itertools::EitherOrBoth;
use unicode_segmentation::UnicodeSegmentation;
use acrostic_core::letter::Letter;
use any_ascii::any_ascii;

use crate::puzzle::Puzzle;



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