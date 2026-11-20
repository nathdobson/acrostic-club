// Tests for the passes that reformat a quote's text, which the solved puzzle shows.

use acrostic_generator::reformat::characters::repair_characters;

#[test]
fn invisible_formatting_characters_vanish_without_splitting_a_word() {
    // A zero-width space.
    assert_eq!(repair_characters("some\u{200b}thing"), "something");
    assert_eq!(repair_characters("o-\u{200b}u-\u{200b}i"), "o-u-i");
    // A soft hyphen.
    assert_eq!(repair_characters("beauti\u{ad}ful"), "beautiful");
    // A byte-order mark and a left-to-right mark.
    assert_eq!(repair_characters("\u{feff}\u{200e}Hello"), "Hello");
    // A variation selector on its own.
    assert_eq!(repair_characters("later.\u{fe0f}"), "later.");
    // A `¬` in place of a soft hyphen.
    assert_eq!(repair_characters("unpro¬cessed"), "unprocessed");
    // A combining grapheme joiner, on a dot.
    assert_eq!(repair_characters("truth.\u{34f} Then"), "truth. Then");
}

#[test]
fn windows_1252_punctuation_decoded_as_latin_1_becomes_the_punctuation_meant() {
    // U+0092 for a right single quotation mark, serving as an apostrophe.
    assert_eq!(repair_characters("don\u{92}t"), "don’t");
    // U+0097 for an em dash.
    assert_eq!(repair_characters("fall\u{97}and"), "fall—and");
    // U+0093 and U+0094 for double quotation marks.
    assert_eq!(
        repair_characters("\u{93}Strange,\u{94} he said"),
        "“Strange,” he said"
    );
    // U+009D, left over beside a closing quotation mark.
    assert_eq!(repair_characters("fear”\u{9d} always"), "fear” always");
}

#[test]
fn a_control_character_becomes_a_space() {
    // The control character stands in for a lost line break.
    assert_eq!(repair_characters("you will.\u{3}If"), "you will. If");
}

#[test]
fn a_fraction_character_becomes_digits_and_a_slash() {
    assert_eq!(repair_characters("the 12 ½ years"), "the 12 1/2 years");
    assert_eq!(repair_characters("a ¾ oz"), "a 3/4 oz");
}

#[test]
fn a_fraction_character_after_a_digit_stays_apart_from_it() {
    assert_eq!(repair_characters("Platform 9¾"), "Platform 9 3/4");
}
