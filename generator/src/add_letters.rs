use crate::puzzle::Puzzle;
use crate::string::{Grapheme, GraphemeString, LetterString};
use itertools::Itertools;
use std::io;
use std::io::ErrorKind;

// A stretch of a quote that is either all letters and digits or all punctuation and
// whitespace.
enum Run {
    // The cells of a word or a number.
    Word(String),
    // The punctuation and whitespace between words.
    Marks(Vec<Mark>),
}

// The kinds of mark that the cell layout tells apart.
#[derive(Clone, Copy)]
enum Mark {
    // Whitespace, which shows as a space.
    Space,
    // A dash, which shows as a hyphen.
    Dash,
    Dot,
    Comma,
    Colon,
    // An apostrophe, a bracket, or one of the few symbols that can sit inside a word.
    InsideWord,
    // Any other punctuation, which shows no cell.
    Other,
}

impl Mark {
    // Classifies a grapheme between words. Fails on an unrecognized mark, since no cells are
    // defined for it.
    //
    // The lists name each variant of a mark, such as each kind of dash, rather than matching
    // marks by their ASCII form: `any_ascii` gives some dingbats and damaged characters the ASCII
    // form of a mark, such as `*` for `★` and `?` for `�`, and those should fail the puzzle
    // rather than pass as the substituted mark.
    fn from_grapheme(grapheme: &Grapheme) -> Result<Mark, String> {
        let mark = grapheme.string();
        if mark.chars().all(char::is_whitespace) {
            return Ok(Mark::Space);
        }
        let kind = match mark {
            // A hyphen, the dashes, a minus sign, and horizontal lines.
            "-" | "‐" | "‑" | "‒" | "–" | "—" | "―" | "−" | "─" | "⎯" | "⎼" => {
                Mark::Dash
            }
            "." => Mark::Dot,
            "," => Mark::Comma,
            ":" => Mark::Colon,
            // Apostrophes and the marks that serve as one, such as a prime; brackets; and a few
            // symbols.
            "'" | "‘" | "’" | "´" | "ʼ" | "ʹ" | "′" | "(" | ")" | "[" | "]" | "&" | "*" | "$" => {
                Mark::InsideWord
            }
            // Quotation marks and the marks that serve as one, such as a diaeresis; Spanish
            // inverted marks; and other punctuation.
            "\"" | "“" | "”" | "‟" | "ʺ" | "¨" | "❝" | "❞" | ";" | "!" | "¡" | "?" | "¿" | "…"
            | "~" | "∼" | "≈" | "_" | "‗" | "/" | "#" => Mark::Other,
            // A Greek question mark, which looks just like a semicolon, so it's written as its
            // code point.
            "\u{37e}" => Mark::Other,
            _ => {
                let unknown_mark = quoted_with_code_points(mark);
                return Err(format!("no cells are defined for the mark {unknown_mark}"));
            }
        };
        Ok(kind)
    }
}

// Splits a quote into runs, which alternate between words and marks. Fails on a mark that no
// cells are defined for.
fn quote_to_runs(graphemes: &GraphemeString) -> Result<Vec<Run>, String> {
    graphemes
        .graphemes()
        .chunk_by(|grapheme| is_part_of_word(grapheme))
        .into_iter()
        .map(|(is_word, run)| {
            if is_word {
                Ok(Run::Word(run.map(word_cells).collect()))
            } else {
                let marks = run.map(Mark::from_grapheme).collect::<Result<_, _>>()?;
                Ok(Run::Marks(marks))
            }
        })
        .collect()
}

// A grapheme for an error message, quoted and followed by its code points, since many of the
// characters that fail are invisible or easily mistaken for others: `"☥" (U+2625)`.
fn quoted_with_code_points(grapheme: &str) -> String {
    let code_points = grapheme
        .chars()
        .map(|character| {
            let code_point = u32::from(character);
            format!("U+{code_point:04X}")
        })
        .join(" ");
    format!("{grapheme:?} ({code_points})")
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

// Rewrites the characters of a quote that don't show what they stand for, before the quote is
// split into graphemes.
fn repair_characters(quote: &str) -> String {
    quote
        .chars()
        .filter_map(|character| match character {
            // Characters that carry no text are removed, so that a word they sit inside stays
            // whole.
            //
            // A soft hyphen.
            '\u{ad}'
            // A `¬`, which the pinned quotes hold only in place of a soft hyphen at an old line
            // break, as in `unpro¬cessed`.
            | '¬'
            // A combining grapheme joiner.
            | '\u{34f}'
            // A zero-width space, non-joiner, and joiner, and left-to-right and right-to-left
            // marks.
            | '\u{200b}'..='\u{200f}'
            // Direction embeddings and overrides.
            | '\u{202a}'..='\u{202e}'
            // A word joiner, invisible math operators, direction isolates, and deprecated
            // formatting characters.
            | '\u{2060}'..='\u{206f}'
            // Variation selectors, which choose how the character before them is drawn.
            | '\u{fe00}'..='\u{fe0f}'
            // A byte-order mark.
            | '\u{feff}' => None,
            // Windows-1252 punctuation decoded as if it were Latin-1 becomes the character it
            // was meant to be.
            '\u{92}' => Some('’'),
            '\u{93}' => Some('“'),
            '\u{94}' => Some('”'),
            '\u{97}' => Some('—'),
            // A byte that Windows-1252 leaves undefined. The pinned quotes hold it only as a
            // leftover just after a `”`, so it stands for nothing.
            '\u{9d}' => None,
            _ => Some(character),
        })
        .collect()
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
fn push_mark_cells(cells: &mut String, mark: Mark) {
    match mark {
        Mark::Space => {
            if !cells.ends_with(' ') {
                cells.push(' ');
            }
        }
        Mark::Dot | Mark::Comma | Mark::Colon | Mark::InsideWord | Mark::Other => {}
        Mark::Dash => cells.push('-'),
    }
}

// Whether marks that show no cell sit inside one word, rather than between two words.
//
// Apostrophes, brackets, and a few symbols sit inside words: `don't`, `[T]he`, `f*ck`,
// `AT&T`. So does a dot between single letters, as in `U.S.A.`, and a comma, dot, or colon
// inside a number, as in `1,000` or `John 3:16`. Other punctuation between two words stands
// in for a space or line break that the quote's text lost, as in `end.Begin` or `book#3`.
fn is_inside_word(marks: &[Mark], word_before: &str, word_after: &str) -> bool {
    let is_single_letter = |word: &str| {
        word.len() == 1
            && word
                .chars()
                .all(|character| character.is_ascii_alphabetic())
    };
    let is_inside_number = word_before.ends_with(|character: char| character.is_ascii_digit())
        && word_after.starts_with(|character: char| character.is_ascii_digit());
    match marks {
        [Mark::Dot] if is_single_letter(word_before) && is_single_letter(word_after) => true,
        [Mark::Dot | Mark::Comma | Mark::Colon] if is_inside_number => true,
        _ => marks.iter().all(|mark| matches!(mark, Mark::InsideWord)),
    }
}

// Lays a quote out as the puzzle's cells: one per letter or digit, a space between words,
// and a hyphen for each dash. Fails on any character it doesn't recognize, such as `@`,
// since it has no cells to give it.
pub fn quote_to_cells(input: &str) -> io::Result<String> {
    let graphemes = GraphemeString::from_str(&repair_characters(input));
    let runs = quote_to_runs(&graphemes).map_err(|problem| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("{problem} in the quote {input:?}"),
        )
    })?;
    let mut cells = String::new();
    for (index, run) in runs.iter().enumerate() {
        match run {
            Run::Word(word) => cells.push_str(word),
            Run::Marks(marks) => {
                let length_before = cells.len();
                for &mark in marks {
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
