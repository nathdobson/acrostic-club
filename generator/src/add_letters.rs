use crate::puzzle::Puzzle;
use crate::string::{Grapheme, GraphemeString, LetterString};
use itertools::Itertools;
use std::io;
use std::io::ErrorKind;
use unicode_script::{Script, UnicodeScript};

// A stretch of a quote that is either all letters and digits or all punctuation and
// whitespace.
enum Run {
    // The cells of a word or a number.
    Word(String),
    // The punctuation and whitespace between words, along with their ASCII forms, which show
    // as written inside a number.
    Marks { marks: Vec<Mark>, text: String },
}

impl Run {
    // The cells of a word or number, or `None` for marks.
    fn as_word(&self) -> Option<&str> {
        match self {
            Run::Word(word) => Some(word),
            Run::Marks { .. } => None,
        }
    }
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

// Whether a word ends in a digit, so that marks after it touch a number.
fn ends_with_digit(word: &str) -> bool {
    word.ends_with(|character: char| character.is_ascii_digit())
}

// Whether a word starts with a digit, so that marks before it touch a number.
fn starts_with_digit(word: &str) -> bool {
    word.starts_with(|character: char| character.is_ascii_digit())
}

// Whether marks sit inside a number: between two digits, with no whitespace among them, as in
// `John 3:16`, `10^9`, or `1,000`.
fn is_inside_number(marks: &[Mark], word_before: &str, word_after: &str) -> bool {
    ends_with_digit(word_before)
        && starts_with_digit(word_after)
        && !marks.iter().any(|mark| matches!(mark, Mark::Space))
}

// Splits a run of marks into the number symbols at its start, attached to the number before it,
// as `%` is in `17% of`; the marks between; and the number symbols at its end, attached to the
// number after it, as `$` is in `of $5`.
fn split_off_number_symbols(
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

// Lays a quote out as the puzzle's cells: one per letter or digit, a space between words, a
// hyphen for each dash, and any symbols inside a number or attached to one as written, as in
// `3:16` or `17%`. Expects text that `reformat_quote` has reformatted, whose characters show what
// they stand for. Fails on any character it doesn't recognize, such as `☥`, since it has no cells
// to give it.
pub fn quote_to_cells(input: &str) -> io::Result<String> {
    let graphemes = GraphemeString::from_str(input);
    let runs = check_letters_are_latin(&graphemes)
        .and_then(|()| quote_to_runs(&graphemes))
        .map_err(|problem| {
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
