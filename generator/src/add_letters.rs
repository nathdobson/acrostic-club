use crate::puzzle::Puzzle;
use crate::string::{Grapheme, GraphemeString, LetterString};
use any_ascii::any_ascii_char;
use itertools::Itertools;
use std::io;
use std::io::ErrorKind;

// A stretch of a quote that is either all letters and digits or all punctuation and
// whitespace.
enum Run {
    // The cells of a word or a number.
    Word(String),
    // The punctuation and whitespace between words, along with their ASCII forms, which show
    // as written inside a number.
    Marks { marks: Vec<Mark>, text: String },
}

// The kinds of mark that the cell layout tells apart.
#[derive(Clone, Copy)]
enum Mark {
    // Whitespace or a control character, which shows as a space.
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
        // Control characters stand in for a line break that the quote's text lost, as in
        // `you will. \u{3}If`.
        let is_space = |character: char| character.is_whitespace() || character.is_ascii_control();
        if mark.chars().all(is_space) {
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
            // Apostrophes and the marks that serve as one, such as a prime or a backtick in
            // ``you`ll``; brackets and braces; and a few symbols, such as `$` and `%`.
            "'" | "‘" | "’" | "´" | "ʼ" | "ʹ" | "′" | "`" | "(" | ")" | "[" | "]" | "{" | "}"
            | "&" | "*" | "$" | "%" => Mark::InsideWord,
            // A space with a combining acute accent, which is `´` decomposed, serving as an
            // apostrophe. It looks just like `´`, so it's written as its code point.
            " \u{301}" => Mark::InsideWord,
            // Quotation marks and the marks that serve as one, such as a diaeresis, including
            // other languages' marks such as `«`; Spanish inverted marks; and other punctuation.
            "\"" | "“" | "”" | "‟" | "ʺ" | "¨" | "❝" | "❞" | "«" | "»" | "„" | ";" | "!" | "¡"
            | "?" | "¿" | "…" | "~" | "∼" | "≈" | "_" | "‗" | "/" | "#" => Mark::Other,
            // A Greek question mark, which looks just like a semicolon, so it's written as its
            // code point.
            "\u{37e}" => Mark::Other,
            // Operators and separators, including separator dots and bullets.
            "=" | "+" | "×" | "÷" | "^" | "<" | ">" | "|" | "\\" | "@" | "·" | "⸱" | "•" => {
                Mark::Other
            }
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
                let graphemes: Vec<&Grapheme> = run.collect();
                let marks = graphemes
                    .iter()
                    .copied()
                    .map(Mark::from_grapheme)
                    .collect::<Result<_, _>>()?;
                let text = graphemes.iter().copied().map(ascii_or_original).collect();
                Ok(Run::Marks { marks, text })
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
    let mut repaired = String::new();
    for character in quote.chars() {
        match character {
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
            | '\u{feff}' => {}
            // Windows-1252 punctuation decoded as if it were Latin-1 becomes the character it
            // was meant to be.
            '\u{92}' => repaired.push('’'),
            '\u{93}' => repaired.push('“'),
            '\u{94}' => repaired.push('”'),
            '\u{97}' => repaired.push('—'),
            // A byte that Windows-1252 leaves undefined. The pinned quotes hold it only as a
            // leftover just after a `”`, so it stands for nothing.
            '\u{9d}' => {}
            _ => {
                // A fraction character becomes digits and a slash, which then show as a number.
                // A space keeps it apart from a whole number just before it, so that `9¾`
                // becomes `9 3/4` rather than `93/4`.
                if let Some(fraction) = fraction_in_ascii(character) {
                    if repaired.ends_with(char::is_numeric) {
                        repaired.push(' ');
                    }
                    repaired.push_str(fraction);
                } else {
                    repaired.push(character);
                }
            }
        }
    }
    repaired
}

// The ASCII form of a fraction character, such as `1/2` for `½`, or `None` for any other
// character.
fn fraction_in_ascii(character: char) -> Option<&'static str> {
    let ascii = any_ascii_char(character);
    let (numerator, denominator) = ascii.split_once('/')?;
    let is_number =
        |text: &str| !text.is_empty() && text.chars().all(|digit| digit.is_ascii_digit());
    (is_number(numerator) && is_number(denominator)).then_some(ascii)
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
// `AT&T`. So does a dot between single letters, as in `U.S.A.` Other punctuation between two
// words stands in for a space or line break that the quote's text lost, as in `end.Begin` or
// `book#3`.
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

// Whether marks sit inside a number: between two digits, with no whitespace among them, as in
// `John 3:16`, `10^9`, or `1,000`.
fn is_inside_number(marks: &[Mark], word_before: &str, word_after: &str) -> bool {
    word_before.ends_with(|character: char| character.is_ascii_digit())
        && word_after.starts_with(|character: char| character.is_ascii_digit())
        && !marks.iter().any(|mark| matches!(mark, Mark::Space))
}

// Lays a quote out as the puzzle's cells: one per letter or digit, a space between words, a
// hyphen for each dash, and any symbols inside a number as written. Fails on any character
// it doesn't recognize, such as `☥`, since it has no cells to give it.
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
            Run::Marks { marks, text } => {
                // Runs alternate, so marks with a run on each side sit between two words.
                let words_around = match (
                    index.checked_sub(1).map(|index_before| &runs[index_before]),
                    runs.get(index + 1),
                ) {
                    (Some(Run::Word(word_before)), Some(Run::Word(word_after))) => {
                        Some((word_before, word_after))
                    }
                    _ => None,
                };
                // Digits show as written, so symbols inside a number do too: dropping them would
                // change the number, as `3:16` would become `316`.
                if let Some((word_before, word_after)) = words_around
                    && is_inside_number(marks, word_before, word_after)
                {
                    cells.push_str(text);
                    continue;
                }
                let length_before = cells.len();
                for &mark in marks {
                    push_mark_cells(&mut cells, mark);
                }
                if let Some((word_before, word_after)) = words_around
                    && cells.len() == length_before
                    && !is_inside_word(marks, word_before, word_after)
                {
                    cells.push(' ');
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
