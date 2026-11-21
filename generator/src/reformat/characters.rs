// Repairs the characters of a quote or its source that don't show what they stand for.

use any_ascii::any_ascii_char;
use encoding_rs::WINDOWS_1252;

// Rewrites the characters of a quote or its source that don't show what they stand for: decodes
// characters garbled by a wrong character encoding, removes those that carry no text, turns control
// characters into spaces, and writes fraction characters as digits and a slash.
pub fn repair_characters(text: &str) -> String {
    let mut repaired = String::new();
    for character in decode_utf8_misread_as_windows_1252(text).chars() {
        match character {
            // Characters that carry no text are removed, so that a word they sit inside stays
            // whole.
            //
            // A soft hyphen.
            '\u{ad}'
            // A `¬`, which the pinned quotes hold only in place of a soft hyphen at an old line
            // break, as in "unpro¬cessed".
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
            // A byte that Windows-1252 leaves undefined. The pinned quotes hold it only as a
            // leftover just after a `”`, so it stands for nothing.
            '\u{9d}' => {}
            // Windows-1252 text decoded as if it were Latin-1 turns the punctuation at bytes 0x80
            // to 0x9F into control characters, such as `\u{92}` for `’`. Each becomes the character
            // it was meant to be.
            '\u{80}'..='\u{9f}' => repaired.push(windows_1252_character(character)),
            // An ASCII control character that isn't whitespace stands in for a line break that the
            // text lost, as in `you will. \u{3}If`, so it becomes a space.
            _ if character.is_ascii_control() && !character.is_whitespace() => repaired.push(' '),
            _ => {
                // A fraction character becomes digits and a slash, which then show as a number. A
                // space keeps it apart from a whole number just before it, so that "9¾" becomes
                // "9 3/4" rather than "93/4".
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

// Decodes the characters of `text` that were stored as UTF-8 and then misread as Windows-1252.
// Misreading garbles each non-ASCII character into one character per byte of its UTF-8: `’` becomes
// "â€™", and `á` becomes "Ã¡". Writing the garbled characters back out as Windows-1252 recovers the
// UTF-8 bytes, which then decode to the character meant.
fn decode_utf8_misread_as_windows_1252(text: &str) -> String {
    // Garbled characters are all non-ASCII, so ASCII text, which most quotes are, holds none.
    if text.is_ascii() {
        return text.to_string();
    }
    // Windows-1252 leaves the last byte of `”` undefined, so some misreadings put a replacement
    // character `�` in its place, which no decoding can reverse. Every "â€�" in the pinned quotes
    // closes a quotation, as in "Quiet,â€� she hissed", so each stands for `”`.
    let text = text.replace("â€\u{fffd}", "”");
    let characters: Vec<char> = text.chars().collect();
    let mut decoded = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        match decode_misread_character(&characters[index..]) {
            Some(character) => {
                decoded.push(character);
                // The garbled form spends one character on each byte of the character's UTF-8.
                index += character.len_utf8();
            }
            None => {
                decoded.push(characters[index]);
                index += 1;
            }
        }
    }
    decoded
}

// Decodes the garbled character that `characters` starts with, giving the character meant, or
// `None` when `characters` starts with no garbled character: `’` for the characters of "â€™s".
fn decode_misread_character(characters: &[char]) -> Option<char> {
    // A character's UTF-8 spans at most four bytes.
    let bytes: Vec<u8> = characters
        .iter()
        .take(4)
        .map_while(|&character| windows_1252_byte(character))
        .collect();
    // The bytes' valid UTF-8 prefix starts with the character meant, if there is one.
    let character = bytes.utf8_chunks().next()?.valid().chars().next()?;
    is_plausibly_misread(character).then_some(character)
}

// Whether `character`, decoded from what looks like garbled text, is one that misreading plausibly
// garbled: a letter whose UTF-8 takes two bytes, such as `á` or the Arabic `ق`, or a punctuation
// mark or symbol from U+2000 to U+2BFF, such as `’` or `€`. Text that only looks garbled can decode
// by coincidence, but into other characters: the "í…—" of the Spanish "así…—Nunca" decodes to the
// Korean syllable `텗`, and the "Í’" of "Í’m" to a combining mark.
fn is_plausibly_misread(character: char) -> bool {
    let is_two_byte_letter = character.len_utf8() == 2 && character.is_alphabetic();
    let is_punctuation_or_symbol = ('\u{2000}'..='\u{2bff}').contains(&character);
    is_two_byte_letter || is_punctuation_or_symbol
}

// The byte that Windows-1252 writes `character` as. Gives `None` for a character that Windows-1252
// can't write, and for an ASCII character, since ASCII bytes never appear inside the UTF-8 of a
// non-ASCII character.
fn windows_1252_byte(character: char) -> Option<u8> {
    if character.is_ascii() {
        return None;
    }
    let mut buffer = [0; 4];
    let (bytes, _, has_unmappable_characters) =
        WINDOWS_1252.encode(character.encode_utf8(&mut buffer));
    match (has_unmappable_characters, bytes.as_ref()) {
        (false, &[byte]) => Some(byte),
        _ => None,
    }
}

// The character that Windows-1252 reads at the byte where Latin-1 reads `character`, a control
// character from U+0080 to U+009F: `’` for `\u{92}`.
fn windows_1252_character(character: char) -> char {
    let bytes = [u8::try_from(character).expect("a character below U+0100 should fit in a byte")];
    let (decoded, _) = WINDOWS_1252.decode_without_bom_handling(&bytes);
    decoded
        .chars()
        .next()
        .expect("Windows-1252 should read every byte as a character")
}

// The ASCII form of a fraction character, such as "1/2" for `½`, or `None` for any other character.
fn fraction_in_ascii(character: char) -> Option<&'static str> {
    let ascii = any_ascii_char(character);
    let (numerator, denominator) = ascii.split_once('/')?;
    let is_number =
        |text: &str| !text.is_empty() && text.chars().all(|digit| digit.is_ascii_digit());
    (is_number(numerator) && is_number(denominator)).then_some(ascii)
}
