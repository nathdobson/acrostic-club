// Splits a quote's text into words and the runs of marks between them, and tells the kinds of mark
// apart. Stage 0 reads the text this way to find where it lost spaces, and stage 1 to lay it out as
// cells.

use crate::string::{Grapheme, GraphemeString};
use itertools::Itertools;
use std::io;
use std::io::ErrorKind;
use std::ops::Range;
use unicode_script::{Script, UnicodeScript};

// A stretch of a quote that is either all letters and digits or all punctuation and whitespace.
pub enum Run {
    // The cells of a word or a number.
    Word(String),
    // The punctuation and whitespace between words.
    Marks {
        // The kind of each mark.
        marks: Vec<Mark>,
        // The marks' ASCII forms, which show as written inside a number, as in `3:16`.
        text: String,
        // The byte offset in the whole quote where each mark starts, followed by the offset just
        // past the run: `[3, 4]` for the `.` in `end.Begin`.
        mark_offsets: Vec<usize>,
    },
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

// The cells of the words on either side of the run at `index`, each `None` at an end of the quote.
// Runs alternate, so a run on either side of marks is a word.
pub fn words_beside(runs: &[Run], index: usize) -> (Option<&str>, Option<&str>) {
    let word_before = index
        .checked_sub(1)
        .and_then(|index_before| runs[index_before].as_word());
    let word_after = runs.get(index + 1).and_then(Run::as_word);
    (word_before, word_after)
}

// The kinds of mark that restoring lost spaces and laying out cells tell apart.
#[derive(Clone, Copy)]
pub enum Mark {
    // Whitespace, which shows as a space.
    Space,
    // A dash, which shows as a hyphen.
    Dash,
    // A slash, which joins words on purpose, as in `and/or`, and shows as a space between them.
    // Inside a number, as in `1/2`, it shows as written.
    Slash,
    // A period, which can sit inside an initialism, as in `U.S.A.`
    Dot,
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
    // The lists name each variant of a mark, such as each kind of dash, rather than matching marks
    // by their ASCII form: `any_ascii` gives some dingbats and damaged characters the ASCII form of
    // a mark, such as `*` for `★` and `?` for `�`, and those should fail the puzzle rather than
    // pass as the substituted mark.
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
            "/" => Mark::Slash,
            "." => Mark::Dot,
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
            // Quotation marks and the marks that serve as one, such as a diaeresis, including other
            // languages' marks such as `«`; Spanish inverted marks; and other punctuation.
            "\"" | "“" | "”" | "‟" | "ʺ" | "¨" | "❝" | "❞" | "«" | "»" | "„" | "," | ":" | ";"
            | "!" | "¡" | "?" | "¿" | "…" | "~" | "∼" | "≈" | "_" | "‗" => Mark::Other,
            // A Greek question mark, which looks just like a semicolon, so it's written as its code
            // point.
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

    // The cell that the mark shows between two words, if any: a space for whitespace or a slash,
    // and a hyphen for a dash. Any other mark shows no cell, so on its own it leaves the words
    // joined, as the apostrophe does in `don't`. That includes a period: one that ends a sentence
    // normally has a space after it, and one without, as in `end.Begin`, stands in for a space that
    // the text lost.
    pub fn cell_between_words(self) -> Option<char> {
        match self {
            Mark::Space | Mark::Slash => Some(' '),
            Mark::Dash => Some('-'),
            // Listing each kind of mark, rather than matching the rest with `_`, makes the compiler
            // ask what a newly added kind shows.
            Mark::Dot | Mark::InsideWord | Mark::NumberSymbol(_) | Mark::Other => None,
        }
    }
}

// Splits a quote's text into runs, which alternate between words and marks.
//
// Fails on a quote that no puzzle can show: one holding a mark with no cells, such as `☥`; a letter
// outside the Latin script, such as `ж`; or a symbol that would be spelled out as letters, such as
// `❤`, which `any_ascii` writes as `heart`.
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

// Splits a quote into runs, which alternate between words and marks. Fails on a mark that no cells
// are defined for, or on a symbol that would be spelled out as letters.
fn quote_to_runs(graphemes: &GraphemeString) -> Result<Vec<Run>, String> {
    // The byte offset in the text where each grapheme starts, followed by the text's length.
    let mut grapheme_offsets = vec![0];
    let mut offset = 0;
    for grapheme in graphemes.graphemes() {
        offset += grapheme.string().len();
        grapheme_offsets.push(offset);
    }
    graphemes
        .graphemes()
        .enumerate()
        .chunk_by(|(_, grapheme)| is_part_of_word(grapheme))
        .into_iter()
        .map(|(is_word, run)| {
            let (indices, graphemes): (Vec<usize>, Vec<&Grapheme>) = run.unzip();
            if is_word {
                let word = graphemes
                    .into_iter()
                    .map(word_cells)
                    .collect::<Result<_, _>>()?;
                Ok(Run::Word(word))
            } else {
                let marks = graphemes
                    .iter()
                    .copied()
                    .map(Mark::from_grapheme)
                    .collect::<Result<_, _>>()?;
                let text = graphemes.iter().copied().map(ascii_or_original).collect();
                // A run's graphemes sit next to each other, so the offsets of its first grapheme
                // through the grapheme just after it give where each mark starts and where the run
                // ends.
                let first_index = *indices
                    .first()
                    .expect("a run should hold at least one grapheme");
                let last_index = *indices
                    .last()
                    .expect("a run should hold at least one grapheme");
                let mark_offsets = grapheme_offsets[first_index..=last_index + 1].to_vec();
                Ok(Run::Marks {
                    marks,
                    text,
                    mark_offsets,
                })
            }
        })
        .collect()
}

// Fails on a grapheme holding a letter outside the Latin script, such as `ж` or `ع`. Its quote is
// written in another script, which `any_ascii` would turn into a transliteration, or into no cells
// at all where it has no ASCII form, as for `ا`.
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

// Where a run of marks sits among the words on either side of it, which decides what each of its
// marks shows.
pub enum MarksPlacement {
    // Inside a number, as the `:` in `John 3:16`, where all the marks show as written: dropping
    // them would change the number, as `3:16` would become `316`.
    InsideNumber,
    // Outside a number. A symbol beside a number still shows as written, for the same reason: `99%`
    // would otherwise become `99`.
    OutsideNumber {
        // The symbols beside the number just before the run, as the `%` in `17% of`.
        number_suffix: String,
        // The positions within the run of the marks between the number symbols on either side:
        // `1..2`, the space, for `17% of`.
        between: Range<usize>,
        // The symbols beside the number just after the run, as the `$` in `of $5`.
        number_prefix: String,
    },
}

// Places a run of marks among the cells of the words on either side of it, each `None` at an end of
// the quote: inside a number, as in `John 3:16`, or outside one, with the number symbols beside a
// number split off, as in `17% of $5`.
pub fn place_marks(
    marks: &[Mark],
    word_before: Option<&str>,
    word_after: Option<&str>,
) -> MarksPlacement {
    let is_digit = |character: char| character.is_ascii_digit();
    let follows_number = word_before.is_some_and(|word| word.ends_with(is_digit));
    let precedes_number = word_after.is_some_and(|word| word.starts_with(is_digit));

    // Marks between two digits, with no whitespace among them, sit inside a number, as in
    // `John 3:16`, `10^9`, or `1,000`.
    let has_space = marks.iter().any(|mark| matches!(mark, Mark::Space));
    if follows_number && precedes_number && !has_space {
        return MarksPlacement::InsideNumber;
    }

    let mut between = 0..marks.len();

    // Number symbols at the run's start belong to the number before it, as `%` does in `17% of`.
    let mut number_suffix = String::new();
    if follows_number {
        while let Some(Mark::NumberSymbol(symbol)) = marks[between.clone()].first() {
            number_suffix.push(*symbol);
            between.start += 1;
        }
    }

    // Number symbols at the run's end belong to the number after it, as `$` does in `of $5`.
    let mut number_prefix = String::new();
    if precedes_number {
        while let Some(Mark::NumberSymbol(symbol)) = marks[between.clone()].last() {
            number_prefix.insert(0, *symbol);
            between.end -= 1;
        }
    }

    MarksPlacement::OutsideNumber {
        number_suffix,
        between,
        number_prefix,
    }
}
