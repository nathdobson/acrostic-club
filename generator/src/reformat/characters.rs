// Repairs the characters of a quote's text that don't show what they stand for.

use any_ascii::any_ascii_char;

// Rewrites the characters of a quote's text that don't show what they stand for: removes those that
// carry no text, turns control characters into spaces, decodes punctuation garbled by a wrong
// character encoding, and writes fraction characters as digits and a slash.
pub fn repair_characters(quote: &str) -> String {
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
            // Windows-1252 punctuation decoded as if it were Latin-1 becomes the character it was
            // meant to be.
            '\u{92}' => repaired.push('’'),
            '\u{93}' => repaired.push('“'),
            '\u{94}' => repaired.push('”'),
            '\u{97}' => repaired.push('—'),
            // A byte that Windows-1252 leaves undefined. The pinned quotes hold it only as a
            // leftover just after a `”`, so it stands for nothing.
            '\u{9d}' => {}
            // An ASCII control character that isn't whitespace stands in for a line break that the
            // quote's text lost, as in `you will. \u{3}If`, so it becomes a space.
            _ if character.is_ascii_control() && !character.is_whitespace() => repaired.push(' '),
            _ => {
                // A fraction character becomes digits and a slash, which then show as a number. A
                // space keeps it apart from a whole number just before it, so that `9¾` becomes
                // `9 3/4` rather than `93/4`.
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

// The ASCII form of a fraction character, such as `1/2` for `½`, or `None` for any other character.
fn fraction_in_ascii(character: char) -> Option<&'static str> {
    let ascii = any_ascii_char(character);
    let (numerator, denominator) = ascii.split_once('/')?;
    let is_number =
        |text: &str| !text.is_empty() && text.chars().all(|digit| digit.is_ascii_digit());
    (is_number(numerator) && is_number(denominator)).then_some(ascii)
}
