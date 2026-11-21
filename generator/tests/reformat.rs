// Tests for the passes that reformat a quote's text, which the solved puzzle shows.

use acrostic_generator::reformat::characters::repair_characters;
use acrostic_generator::reformat::lost_spaces::restore_lost_spaces;
use acrostic_generator::reformat::reformat_quote;

#[test]
fn reformatting_repairs_characters_before_restoring_lost_spaces() {
    // The zero-width space hides the lost space after "end." until the repair removes it.
    let reformatted_quote =
        reformat_quote("end.\u{200b}Begin").expect("the quote should lay out as cells");

    assert_eq!(reformatted_quote, "end. Begin");
}

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
fn utf8_misread_as_windows_1252_becomes_the_character_meant() {
    // A punctuation mark, whose UTF-8 takes three bytes.
    assert_eq!(repair_characters("motherâ€™s diary"), "mother’s diary");
    // Accented letters, whose UTF-8 takes two bytes.
    assert_eq!(repair_characters("BahÃ¡'u'llÃ¡h"), "Bahá'u'lláh");
    assert_eq!(repair_characters("Danny MekiÄ‡"), "Danny Mekić");
    // Letters of another alphabet.
    assert_eq!(repair_characters("Ø§Ù„Ù‚Ø±Ø¢Ù†"), "القرآن");
}

#[test]
fn a_misread_closing_quotation_mark_missing_its_last_byte_becomes_the_mark() {
    assert_eq!(
        repair_characters("Quiet,â€\u{fffd} she hissed"),
        "Quiet,” she hissed"
    );
}

#[test]
fn text_that_only_looks_misread_is_kept() {
    // "í…—" would decode to the Korean syllable `텗`.
    assert_eq!(repair_characters("así…—Nunca"), "así…—Nunca");
    // "Í’" would decode to a combining mark.
    assert_eq!(
        repair_characters("“Í’m not certain,”"),
        "“Í’m not certain,”"
    );
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

// The quote with its lost spaces restored, for a quote that should lay out as cells.
fn spaces_restored(quote: &str) -> String {
    restore_lost_spaces(quote).expect("the quote should lay out as cells")
}

#[test]
fn a_quote_without_lost_spaces_is_unchanged() {
    assert_eq!(
        spaces_restored("Carpe diem, quam minimum credula postero."),
        "Carpe diem, quam minimum credula postero."
    );
}

#[test]
fn punctuation_standing_in_for_a_lost_space_gains_a_space() {
    assert_eq!(spaces_restored("end.Begin"), "end. Begin");
    assert_eq!(spaces_restored("red,green"), "red, green");
    assert_eq!(spaces_restored("as follows:First"), "as follows: First");
    assert_eq!(spaces_restored("wait...then"), "wait... then");
    assert_eq!(spaces_restored("wait…then"), "wait… then");
    // One line of dialogue ending and the next beginning.
    assert_eq!(spaces_restored("end.\"\"Begin"), "end.\" \"Begin");
    assert_eq!(spaces_restored("why?”“Because"), "why?” “Because");
    // A line of underscores between two passages.
    assert_eq!(spaces_restored("end____Begin"), "end____ Begin");
    // A tilde before the author's name.
    assert_eq!(spaces_restored("end~Author"), "end ~Author");
    // Unlike an asterisk, a number sign seldom censors a letter. Between two words, it more often
    // starts a hashtag that lost the space before it.
    assert_eq!(spaces_restored("love#life"), "love #life");
}

#[test]
fn operators_and_separators_between_words_stand_in_for_a_lost_space() {
    assert_eq!(spaces_restored("LAWS>LAWS"), "LAWS> LAWS");
    assert_eq!(spaces_restored("God+Wisdom"), "God+ Wisdom");
    assert_eq!(spaces_restored("fire\\How"), "fire\\ How");
    assert_eq!(spaces_restored("world:•spread"), "world:• spread");
    assert_eq!(spaces_restored("Dream×Vision"), "Dream× Vision");
}

#[test]
fn marks_inside_a_word_or_a_number_stand_in_for_no_space() {
    assert_eq!(spaces_restored("don't"), "don't");
    assert_eq!(spaces_restored("U.S.A. Today"), "U.S.A. Today");
    assert_eq!(spaces_restored("John 3:16"), "John 3:16");
}

#[test]
fn a_symbol_beside_a_number_run_into_a_word_stands_in_for_a_lost_space() {
    assert_eq!(spaces_restored("17%of"), "17% of");
    assert_eq!(spaces_restored("book#3"), "book #3");
}

#[test]
fn a_slash_between_words_gets_no_space() {
    // The slash already keeps the words apart, as the cells show it as a space.
    assert_eq!(spaces_restored("and/or"), "and/or");
}

#[test]
fn a_lost_space_goes_before_an_opening_curly_quotation_mark() {
    assert_eq!(spaces_restored("He said,“Go now.”"), "He said, “Go now.”");
}

#[test]
fn a_straight_mark_after_an_opening_one_closes() {
    // The first mark opens, since it starts the quote, so the second closes.
    assert_eq!(spaces_restored("\"Go,\"he said."), "\"Go,\" he said.");
}

#[test]
fn a_straight_mark_after_a_closing_one_opens() {
    // The space after the second mark shows that it closes, so the third opens.
    assert_eq!(
        spaces_restored("\"Go,\" he said.\"Now.\""),
        "\"Go,\" he said. \"Now.\""
    );
}

#[test]
fn side_by_side_straight_marks_close_then_open() {
    assert_eq!(spaces_restored("Is it?\"\"Yes."), "Is it?\" \"Yes.");
}

#[test]
fn a_straight_mark_before_a_comma_closes() {
    assert_eq!(
        spaces_restored("no pretending\",Jace said"),
        "no pretending\", Jace said"
    );
}

#[test]
fn an_apostrophe_inside_a_word_plays_no_part_in_a_quotation() {
    // Were the apostrophe in "Can't" to close the quotation, the mark after "stop," would open
    // another.
    assert_eq!(
        spaces_restored("'Can't stop,'he said."),
        "'Can't stop,' he said."
    );
}

#[test]
fn a_quote_that_no_puzzle_can_show_fails() {
    let error =
        restore_lost_spaces("Healed ☥ at last.").expect_err("a quote holding `☥` should fail");

    assert!(error.to_string().contains("U+2625"), "{error}");
}
