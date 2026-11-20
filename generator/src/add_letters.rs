use crate::puzzle::Puzzle;
use crate::string::LetterString;
use crate::words_and_marks::{
    Mark, Run, ends_with_digit, is_inside_number, split_into_runs, split_off_number_symbols,
    starts_with_digit,
};
use std::io;

// Appends the cells a mark shows as: a space for whitespace, unless the cells already end
// in one, and a hyphen for a dash. Other punctuation shows no cell.
fn push_mark_cells(cells: &mut String, mark: Mark) {
    match mark {
        Mark::Space => {
            if !cells.ends_with(' ') {
                cells.push(' ');
            }
        }
        Mark::Dot
        | Mark::Comma
        | Mark::Colon
        | Mark::InsideWord
        | Mark::NumberSymbol(_)
        | Mark::Other => {}
        Mark::Dash => cells.push('-'),
    }
}

// Whether marks that show no cell sit inside one word, rather than between two words.
//
// Apostrophes, brackets, and a few symbols sit inside words: `don't`, `[T]he`, `f*ck`,
// `AT&T`. So does a dot between single letters, as in `U.S.A.` Other punctuation between two
// words stands in for a space or line break that the quote's text lost, as in `end.Begin` or
// `love#life`.
fn is_inside_word(marks: &[Mark], word_before: &str, word_after: &str) -> bool {
    let is_single_letter = |word: &str| {
        word.len() == 1
            && word
                .chars()
                .all(|character| character.is_ascii_alphabetic())
    };
    match marks {
        [Mark::Dot] if is_single_letter(word_before) && is_single_letter(word_after) => true,
        _ => marks.iter().all(|mark| matches!(mark, Mark::InsideWord)),
    }
}

// Lays a quote out as the puzzle's cells: one per letter or digit, a space between words, a
// hyphen for each dash, and any symbols inside a number or attached to one as written, as in
// `3:16` or `17%`. Expects text that `reformat_quote` has reformatted, whose characters show what
// they stand for. Fails on any character it doesn't recognize, such as `☥`, since it has no cells
// to give it.
pub fn quote_to_cells(input: &str) -> io::Result<String> {
    let runs = split_into_runs(input)?;
    let mut cells = String::new();
    for (index, run) in runs.iter().enumerate() {
        match run {
            Run::Word(word) => cells.push_str(word),
            Run::Marks { marks, text } => {
                // Runs alternate, so a run on either side of marks is a word.
                let word_before = index
                    .checked_sub(1)
                    .and_then(|index_before| runs[index_before].as_word());
                let word_after = runs.get(index + 1).and_then(Run::as_word);
                // Digits show as written, so symbols inside a number do too: dropping them would
                // change the number, as `3:16` would become `316`.
                if let (Some(word_before), Some(word_after)) = (word_before, word_after)
                    && is_inside_number(marks, word_before, word_after)
                {
                    cells.push_str(text);
                    continue;
                }
                // A number symbol shows as written beside its number, for the same reason: `99%`
                // would otherwise become `99`.
                let (number_suffix, between, number_prefix) = split_off_number_symbols(
                    marks,
                    word_before.is_some_and(ends_with_digit),
                    word_after.is_some_and(starts_with_digit),
                );
                cells.push_str(&number_suffix);
                // The remaining marks show cells only between two words: at either end of the
                // quote, they separate nothing.
                if let (Some(word_before), Some(word_after)) = (word_before, word_after) {
                    let length_before = cells.len();
                    for &mark in between {
                        push_mark_cells(&mut cells, mark);
                    }
                    // With no marks left between them, the words are separate: a number symbol ran
                    // straight into the next word, as in `17%of`, where the quote's text is
                    // missing a space.
                    let are_separate_words =
                        between.is_empty() || !is_inside_word(between, word_before, word_after);
                    if cells.len() == length_before && are_separate_words {
                        cells.push(' ');
                    }
                }
                cells.push_str(&number_prefix);
            }
        }
    }
    Ok(cells)
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
