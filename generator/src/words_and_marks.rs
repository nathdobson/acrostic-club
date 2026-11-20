// Splits a quote's text into words and the runs of marks between them, and tells the kinds of
// mark apart, for laying the text out as cells.

use crate::string::{Grapheme, GraphemeString};
use itertools::Itertools;
use std::io;
use std::io::ErrorKind;
use unicode_script::{Script, UnicodeScript};

// A stretch of a quote that is either all letters and digits or all punctuation and
// whitespace.
pub enum Run {
    // The cells of a word or a number.
    Word(String),
    // The punctuation and whitespace between words, along with their ASCII forms, which show
    // as written inside a number.
    Marks { marks: Vec<Mark>, text: String },
}

impl Run {
    // The cells of a word or number, or `None` for marks.
    pub fn as_word(&self) -> Option<&str> {
        match self {
            Run::Word(word) => Some(word),
            Run::Marks { .. } => None,
        }
    }
}

// The kinds of mark that the cell layout tells apart.
#[derive(Clone, Copy)]
pub enum Mark {
    // Whitespace, which shows as a space.
    Space,
    // A dash, which shows as a hyphen.
    Dash,
    Dot,
    Comma,
    Colon,
    // An apostrophe, a bracket, or one of the few symbols that can sit inside a word.
    InsideWord,
    // A symbol that belongs to a number it touches, as in `17%`, `$5`, `#1`, or `6+`, and shows as
    // written there. Elsewhere, it acts as other punctuation.
    NumberSymbol(char),
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
            // Apostrophes and the marks that serve as one, such as a prime or a backtick in
            // ``you`ll``; brackets and braces; `&`, which joins the letters of an initialism such
            // as `R&B`; and `*`, which censors a letter, as in `f*ck`.
            "'" | "‘" | "’" | "´" | "ʼ" | "ʹ" | "′" | "`" | "(" | ")" | "[" | "]" | "{" | "}"
            | "&" | "*" => Mark::InsideWord,
            "%" | "$" | "#" | "+" => Mark::NumberSymbol(
                mark.parse()
                    .expect("each number symbol should be a single character"),
            ),
            // A space with a combining acute accent, which is `´` decomposed, serving as an
            // apostrophe. It looks just like `´`, so it's written as its code point.
            " \u{301}" => Mark::InsideWord,
            // Quotation marks and the marks that serve as one, such as a diaeresis, including
            // other languages' marks such as `«`; Spanish inverted marks; and other punctuation.
            "\"" | "“" | "”" | "‟" | "ʺ" | "¨" | "❝" | "❞" | "«" | "»" | "„" | ";" | "!" | "¡"
            | "?" | "¿" | "…" | "~" | "∼" | "≈" | "_" | "‗" | "/" => Mark::Other,
            // A Greek question mark, which looks just like a semicolon, so it's written as its
            // code point.
            "\u{37e}" => Mark::Other,
            // Operators and separators, including separator dots and bullets.
            "=" | "×" | "÷" | "^" | "<" | ">" | "|" | "\\" | "@" | "·" | "⸱" | "•" => {
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

// Splits a quote's text into runs, which alternate between words and marks. Fails on a
// character that no cells are defined for, such as `☥`.
pub fn split_into_runs(quote: &str) -> io::Result<Vec<Run>> {
    let graphemes = GraphemeString::from_str(quote);
    check_letters_are_latin(&graphemes)
        .and_then(|()| quote_to_runs(&graphemes))
        .map_err(|problem| {
            io::Error::new(
                ErrorKind::InvalidInput,
                format!("{problem} in the quote {quote:?}"),
            )
        })
}

// Splits a quote into runs, which alternate between words and marks. Fails on a mark that no
// cells are defined for, or on a symbol that would be spelled out as letters.
fn quote_to_runs(graphemes: &GraphemeString) -> Result<Vec<Run>, String> {
    graphemes
        .graphemes()
        .chunk_by(|grapheme| is_part_of_word(grapheme))
        .into_iter()
        .map(|(is_word, run)| {
            if is_word {
                let word = run.map(word_cells).collect::<Result<_, _>>()?;
                Ok(Run::Word(word))
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

// Fails on a grapheme holding a letter outside the Latin script, such as `ж` or `ع`. Its quote
// is written in another script, which `any_ascii` would turn into a transliteration, or into no
// cells at all where it has no ASCII form, as for `ا`.
fn check_letters_are_latin(graphemes: &GraphemeString) -> Result<(), String> {
    // Diacritics belong to the inherited script, and some letter-like marks, such as `ʹ`, to the
    // script that many writing systems share.
    let is_latin_or_shared = |character: char| {
        matches!(
            character.script(),
            Script::Latin | Script::Common | Script::Inherited
        )
    };
    for grapheme in graphemes.graphemes() {
        let is_foreign_letter =
            |character: char| character.is_alphabetic() && !is_latin_or_shared(character);
        if grapheme.string().chars().any(is_foreign_letter) {
            let letter = quoted_with_code_points(grapheme.string());
            return Err(format!(
                "the letter {letter} is outside the Latin script, so the cells could show it only \
                 by transliterating it into letters the text doesn't hold, and no cells are \
                 defined for it"
            ));
        }
    }
    Ok(())
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
//
// Fails on a symbol that `any_ascii` spells out as letters, such as `heart` for `❤` or `TM` for
// `™`. The quote doesn't hold those letters, and the symbol may be vital to it, as in `I ❤ NY`.
fn word_cells(grapheme: &Grapheme) -> Result<String, String> {
    if grapheme.letters().is_empty() {
        return Ok(ascii_or_original(grapheme).to_string());
    }
    let letters: String = grapheme
        .letters()
        .iter()
        .map(|letter| letter.to_char())
        .collect();
    if !grapheme.string().chars().any(char::is_alphanumeric) {
        let symbol = quoted_with_code_points(grapheme.string());
        return Err(format!(
            "the symbol {symbol} would be spelled out as the letters {letters:?}, which the text \
             doesn't hold, so no cells are defined for it"
        ));
    }
    Ok(letters)
}

// The grapheme in its ASCII form, or as written when it has none.
fn ascii_or_original(grapheme: &Grapheme) -> &str {
    if grapheme.ascii().is_empty() {
        grapheme.string()
    } else {
        grapheme.ascii()
    }
}

// Whether a word ends in a digit, so that marks after it touch a number.
pub fn ends_with_digit(word: &str) -> bool {
    word.ends_with(|character: char| character.is_ascii_digit())
}

// Whether a word starts with a digit, so that marks before it touch a number.
pub fn starts_with_digit(word: &str) -> bool {
    word.starts_with(|character: char| character.is_ascii_digit())
}

// Whether marks sit inside a number: between two digits, with no whitespace among them, as in
// `John 3:16`, `10^9`, or `1,000`.
pub fn is_inside_number(marks: &[Mark], word_before: &str, word_after: &str) -> bool {
    ends_with_digit(word_before)
        && starts_with_digit(word_after)
        && !marks.iter().any(|mark| matches!(mark, Mark::Space))
}

// Splits a run of marks into the number symbols at its start, attached to the number before it,
// as `%` is in `17% of`; the marks between; and the number symbols at its end, attached to the
// number after it, as `$` is in `of $5`.
pub fn split_off_number_symbols(
    marks: &[Mark],
    follows_number: bool,
    precedes_number: bool,
) -> (String, &[Mark], String) {
    let mut between = marks;
    let mut number_suffix = String::new();
    if follows_number {
        while let [Mark::NumberSymbol(symbol), rest @ ..] = between {
            number_suffix.push(*symbol);
            between = rest;
        }
    }
    let mut number_prefix = String::new();
    if precedes_number {
        while let [rest @ .., Mark::NumberSymbol(symbol)] = between {
            number_prefix.insert(0, *symbol);
            between = rest;
        }
    }
    (number_suffix, between, number_prefix)
}
