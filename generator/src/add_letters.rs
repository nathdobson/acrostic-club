use crate::puzzle::Puzzle;
use crate::string::{Grapheme, GraphemeString, LetterString};
use itertools::Itertools;
use std::io;
// pub fn segment(s: &str) -> Vec<EitherOrBoth<Letter, String>> {
//     let mut result = vec![];
//     for grapheme in s.graphemes(true) {
//         let mut letters = vec![];
//         let ascii = any_ascii(grapheme);
//         for c in ascii.bytes() {
//             if let Ok(l) = Letter::new(c) {
//                 letters.push(l);
//             }
//         }
//         if letters.len() == 0 {
//             result.push(EitherOrBoth::Right(grapheme.to_string()));
//         } else {
//             for (index, letter) in letters.iter().enumerate() {
//                 if index == 0 {
//                     result.push(EitherOrBoth::Both(*letter, grapheme.to_string()));
//                 } else {
//                     result.push(EitherOrBoth::Left(*letter))
//                 }
//             }
//         }
//     }
//     result
// }

// A stretch of a quote that is either all letters and digits or all punctuation and
// whitespace.
enum Run<'a> {
    // The cells of a word or a number.
    Word(String),
    // The punctuation and whitespace between words, each mark in its ASCII form where it
    // has one.
    Marks(Vec<&'a str>),
}

// Splits a quote into runs, which alternate between words and marks.
fn quote_to_runs(graphemes: &GraphemeString) -> Vec<Run<'_>> {
    graphemes
        .graphemes()
        .chunk_by(|grapheme| is_part_of_word(grapheme))
        .into_iter()
        .map(|(is_word, run)| {
            if is_word {
                Run::Word(run.map(word_cells).collect())
            } else {
                Run::Marks(run.map(ascii_or_original).collect())
            }
        })
        .collect()
}

// Whether a grapheme belongs to a word or number, rather than to the marks between words.
fn is_part_of_word(grapheme: &Grapheme) -> bool {
    !grapheme.letters().is_empty()
        || ascii_or_original(grapheme)
            .chars()
            .all(|character| character.is_ascii_digit())
}

// The cells a grapheme inside a word shows as: its letters, or the digits it stands for.
fn word_cells(grapheme: &Grapheme) -> String {
    if grapheme.letters().is_empty() {
        ascii_or_original(grapheme).to_string()
    } else {
        grapheme
            .letters()
            .iter()
            .map(|letter| letter.to_char())
            .collect()
    }
}

// The grapheme in its ASCII form, or as written when it has none.
fn ascii_or_original(grapheme: &Grapheme) -> &str {
    if grapheme.ascii().is_empty() {
        grapheme.string()
    } else {
        grapheme.ascii()
    }
}

// Appends the cells a mark shows as: a space for whitespace, unless the cells already end
// in one, and a hyphen for a dash. Other punctuation shows no cell.
fn push_mark_cells(cells: &mut String, mark: &str) {
    match mark {
        " " | "\t" | "\n" | "\u{a0}" => {
            if !cells.ends_with(' ') {
                cells.push(' ');
            }
        }

        "." | "," | ";" | "'" | "\"" | "!" | "?" | "‘" | "’" | ":" | "&" | "*" | "(" | ")"
        | "”" | "“" | "…" | "$" | "~" | "_" | "/" | "´" | "[" | "]" | "#" | "..." => {}

        "-" | "—" | "–" => cells.push('-'),
        _ => panic!("no cells are defined for the mark {mark:?}"),
    }
}

// Whether marks that show no cell sit inside one word, rather than between two words.
//
// Apostrophes, brackets, and a few symbols sit inside words: `don't`, `[T]he`, `f*ck`,
// `AT&T`. So does a dot between single letters, as in `U.S.A.`, and a comma, dot, or colon
// inside a number, as in `1,000` or `John 3:16`. Other punctuation between two words stands
// in for a space or line break that the quote's text lost, as in `end.Begin` or `book#3`.
fn is_inside_word(marks: &[&str], word_before: &str, word_after: &str) -> bool {
    let is_single_letter = |word: &str| {
        word.len() == 1
            && word
                .chars()
                .all(|character| character.is_ascii_alphabetic())
    };
    let is_inside_number = word_before.ends_with(|character: char| character.is_ascii_digit())
        && word_after.starts_with(|character: char| character.is_ascii_digit());
    match marks {
        ["."] if is_single_letter(word_before) && is_single_letter(word_after) => true,
        ["." | "," | ":"] if is_inside_number => true,
        _ => marks.iter().all(|mark| {
            matches!(
                *mark,
                "'" | "‘" | "’" | "´" | "(" | ")" | "[" | "]" | "&" | "*" | "$"
            )
        }),
    }
}

// Lays a quote out as the puzzle's cells: one per letter or digit, a space between words,
// and a hyphen for each dash. Panics on any character it doesn't recognize, such as `@`,
// since it has no cells to give it.
pub fn quote_to_cells(input: &str) -> String {
    let graphemes = GraphemeString::from_str(input);
    let runs = quote_to_runs(&graphemes);
    let mut cells = String::new();
    for (index, run) in runs.iter().enumerate() {
        match run {
            Run::Word(word) => cells.push_str(word),
            Run::Marks(marks) => {
                let length_before = cells.len();
                for mark in marks {
                    push_mark_cells(&mut cells, mark);
                }
                // Runs alternate, so marks with a run on each side sit between two words.
                if let (Some(Run::Word(word_before)), Some(Run::Word(word_after))) = (
                    index.checked_sub(1).map(|index_before| &runs[index_before]),
                    runs.get(index + 1),
                ) {
                    if cells.len() == length_before
                        && !is_inside_word(marks, word_before, word_after)
                    {
                        cells.push(' ');
                    }
                }
            }
        }
    }
    cells
}

pub async fn add_letters(pindex: usize) -> io::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage0.json").await?;
    puzzle
        .quote_letters
        .get_or_insert_with(|| quote_to_cells(&puzzle.quote));
    puzzle.source_letters.get_or_insert_with(|| {
        LetterString::from_str(&puzzle.source)
            .iter()
            .map(|x| x.to_char())
            .collect()
    });
    puzzle.write(pindex, "stage1.json").await?;
    Ok(())
}
