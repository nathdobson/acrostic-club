use crate::puzzle::Puzzle;
use crate::string::LetterString;
use crate::words_and_marks::{
    Mark, MarksPlacement, Run, place_marks, split_into_runs, words_beside,
};
use std::io;

// Lays a quote out as the puzzle's cells: one per letter or digit, a space for whitespace or a
// slash between words, a hyphen for each dash, and any symbols inside a number or beside one as
// written, as in `3:16` or `17%`. Any other mark shows no cell, so the words on either side of it
// join: `don't` gives `DONT`.
//
// Expects text that `reformat_quote` has already formatted. Unreformatted text such as `end.Begin`
// should never arrive; it would lay out as `ENDBEGIN`, without any intervening spaces.
pub fn quote_to_cells(input: &str) -> io::Result<String> {
    let runs = split_into_runs(input)?;
    let mut cells = String::new();
    for (index, run) in runs.iter().enumerate() {
        match run {
            Run::Word(word) => cells.push_str(word),
            Run::Marks { marks, text, .. } => {
                let (word_before, word_after) = words_beside(&runs, index);
                cells.push_str(&marks_to_cells(marks, text, word_before, word_after));
            }
        }
    }
    Ok(cells)
}

// The cells that a run of marks shows, given its text and the cells of the words on either side,
// each `None` at an end of the quote.
fn marks_to_cells(
    marks: &[Mark],
    text: &str,
    word_before: Option<&str>,
    word_after: Option<&str>,
) -> String {
    match place_marks(marks, word_before, word_after) {
        // Inside a number, as in `3:16`, the marks show as written.
        MarksPlacement::InsideNumber => text.to_string(),
        MarksPlacement::OutsideNumber {
            number_suffix,
            between,
            number_prefix,
        } => {
            // At either end of the quote, marks separate no words, so they show no cells: `—Yes.`
            // gives `YES`, not `-YES`. A symbol beside a number still shows, since it belongs to
            // the number: `100%.` gives `100%`.
            let is_between_words = word_before.is_some() && word_after.is_some();
            let between_cells = if is_between_words {
                cells_between_words(&marks[between])
            } else {
                String::new()
            };
            format!("{number_suffix}{between_cells}{number_prefix}")
        }
    }
}

// The cells that marks between two words show: one for each mark that shows a cell, though
// several spaces in a row show as one.
fn cells_between_words(marks: &[Mark]) -> String {
    let mut cells = String::new();
    for cell in marks.iter().filter_map(|mark| mark.cell_between_words()) {
        let is_repeated_space = cell == ' ' && cells.ends_with(' ');
        if !is_repeated_space {
            cells.push(cell);
        }
    }
    cells
}

// Spells out a puzzle's source as its letters, one per answer, transliterated to ASCII: `Émile
// Zola` gives `EMILEZOLA`.
pub fn source_to_letters(source: &str) -> String {
    LetterString::from_str(source)
        .iter()
        .map(|letter| letter.to_char())
        .collect()
}

// Stage 1: lays the quote from stage0.json out as the puzzle's cells, spells out its source as
// letters, and writes the puzzle to stage1.json.
pub async fn add_letters(pindex: usize) -> io::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage0.json").await?;
    if puzzle.quote_letters.is_none() {
        puzzle.quote_letters = Some(quote_to_cells(&puzzle.quote)?);
    }
    puzzle
        .source_letters
        .get_or_insert_with(|| source_to_letters(&puzzle.source));
    puzzle.write(pindex, "stage1.json").await?;
    Ok(())
}
