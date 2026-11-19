// Tests for `quote_to_cells`: how a quote's text becomes the puzzle's cells.

use acrostic_generator::add_letters::quote_to_cells;

// The cells for a quote that should lay out without error.
fn cells_for(quote: &str) -> String {
    quote_to_cells(quote).expect("the quote should lay out as cells")
}

// The error message for a quote that should fail to lay out.
fn error_for(quote: &str) -> String {
    quote_to_cells(quote)
        .expect_err("the quote should fail to lay out")
        .to_string()
}

#[test]
fn an_unknown_mark_fails_naming_its_code_points() {
    let message = error_for("I ☥ you");

    assert!(message.contains("U+2625"), "{message}");
}

#[test]
fn a_symbol_that_any_ascii_spells_as_letters_fails() {
    // `any_ascii` writes these as `:heart:`, `:coffee:`, `TM`, and `L`.
    assert!(error_for("keep flying ❤").contains("U+2764"));
    assert!(error_for("For later. ☕\u{fe0f}").contains("U+2615"));
    assert!(error_for("Brand™ new").contains("U+2122"));
    assert!(error_for("a £5 note").contains("U+00A3"));
}

#[test]
fn a_symbol_spelled_as_letters_fails_naming_the_letters() {
    let message = error_for("keep flying ❤");

    assert!(message.contains("\"HEART\""), "{message}");
}

#[test]
fn a_letter_outside_the_latin_script_fails() {
    // Cyrillic, which `any_ascii` would transliterate.
    assert!(error_for("мир").contains("U+043C"));
    // An Arabic alef, which `any_ascii` would drop.
    assert!(error_for("ا").contains("U+0627"));
}

#[test]
fn latin_letters_with_diacritics_become_plain_letters() {
    assert_eq!(cells_for("café naïve Æsir"), "CAFE NAIVE AESIR");
}

#[test]
fn full_width_digits_and_letters_become_ascii_cells() {
    assert_eq!(cells_for("０１２３４５６７８９"), "0123456789");
    assert_eq!(
        cells_for("ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ"),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    );
    assert_eq!(
        cells_for("ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ"),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    );
}

#[test]
fn punctuation_standing_in_for_a_lost_space_breaks_words() {
    assert_eq!(cells_for("end.Begin"), "END BEGIN");
    assert_eq!(cells_for("red,green"), "RED GREEN");
    assert_eq!(cells_for("and/or"), "AND OR");
    assert_eq!(cells_for("as follows:First"), "AS FOLLOWS FIRST");
    assert_eq!(cells_for("wait...then"), "WAIT THEN");
    assert_eq!(cells_for("wait…then"), "WAIT THEN");
    // One line of dialogue ending and the next beginning.
    assert_eq!(cells_for("end.\"\"Begin"), "END BEGIN");
    assert_eq!(cells_for("why?”“Because"), "WHY BECAUSE");
    // A line of underscores between two passages.
    assert_eq!(cells_for("end____Begin"), "END BEGIN");
    // A tilde before the author's name.
    assert_eq!(cells_for("end~Author"), "END AUTHOR");
    // Unlike an asterisk, a number sign seldom censors a letter. Between two words, it more
    // often starts a hashtag or a number that lost the space before it.
    assert_eq!(cells_for("love#life"), "LOVE LIFE");
    assert_eq!(cells_for("book#3"), "BOOK 3");
}

#[test]
fn tabs_newlines_and_non_breaking_spaces_count_as_spaces() {
    assert_eq!(cells_for("one\ttwo"), "ONE TWO");
    assert_eq!(cells_for("one\ntwo"), "ONE TWO");
    assert_eq!(cells_for("one\u{a0}two"), "ONE TWO");
}

#[test]
fn invisible_formatting_characters_vanish_without_splitting_a_word() {
    // A zero-width space.
    assert_eq!(cells_for("some\u{200b}thing"), "SOMETHING");
    assert_eq!(cells_for("o-\u{200b}u-\u{200b}i"), "O-U-I");
    // A soft hyphen.
    assert_eq!(cells_for("beauti\u{ad}ful"), "BEAUTIFUL");
    // A byte-order mark and a left-to-right mark.
    assert_eq!(cells_for("\u{feff}\u{200e}Hello"), "HELLO");
    // A variation selector on its own.
    assert_eq!(cells_for("later.\u{fe0f}"), "LATER");
    // A `¬` in place of a soft hyphen.
    assert_eq!(cells_for("unpro¬cessed"), "UNPROCESSED");
    // A combining grapheme joiner, on a dot.
    assert_eq!(cells_for("truth.\u{34f} Then"), "TRUTH THEN");
}

#[test]
fn windows_1252_punctuation_decoded_as_latin_1_counts_as_meant() {
    // U+0092 for a right single quotation mark, serving as an apostrophe.
    assert_eq!(cells_for("don\u{92}t"), "DONT");
    // U+0097 for an em dash.
    assert_eq!(cells_for("fall\u{97}and"), "FALL-AND");
    // U+0093 and U+0094 for double quotation marks.
    assert_eq!(cells_for("\u{93}Strange,\u{94} he said"), "STRANGE HE SAID");
    // U+009D, left over beside a closing quotation mark.
    assert_eq!(cells_for("fear”\u{9d} always"), "FEAR ALWAYS");
}

#[test]
fn control_characters_count_as_spaces() {
    assert_eq!(cells_for("you will. \u{3}If"), "YOU WILL IF");
    // A vertical tab and a form feed.
    assert_eq!(cells_for("one\u{b}two\u{c}three"), "ONE TWO THREE");
}

#[test]
fn a_backtick_percent_sign_or_brace_stays_inside_a_word() {
    assert_eq!(cells_for("you`ll"), "YOULL");
    assert_eq!(cells_for("99% of"), "99 OF");
    assert_eq!(cells_for("{T}he"), "THE");
}

#[test]
fn operators_and_separators_between_words_show_no_cell() {
    assert_eq!(cells_for("You + Me"), "YOU ME");
    assert_eq!(cells_for("school= pranks"), "SCHOOL PRANKS");
    assert_eq!(cells_for("LAWS>LAWS"), "LAWS LAWS");
    assert_eq!(cells_for("never <3"), "NEVER 3");
    assert_eq!(cells_for("world | the"), "WORLD THE");
    assert_eq!(cells_for("fire\\How"), "FIRE HOW");
    assert_eq!(cells_for("@bobby i'm"), "BOBBY IM");
    assert_eq!(cells_for("so wrong!^^^ so"), "SO WRONG SO");
}

#[test]
fn other_languages_quotation_marks_show_no_cell() {
    assert_eq!(cells_for("«idealist»"), "IDEALIST");
    assert_eq!(cells_for("„Do we"), "DO WE");
}

#[test]
fn a_dingbat_with_the_ascii_form_of_a_known_symbol_fails() {
    // `any_ascii` writes these as `+`, `+`, `>`, and `=`, which have cells.
    assert!(error_for("healed ☥ She").contains("U+2625"));
    assert!(error_for("prince †1183").contains("U+2020"));
    assert!(error_for("LAWS→LAWS").contains("U+2192"));
    assert!(error_for("a ≡ b").contains("U+2261"));
}

#[test]
fn separator_dots_bullets_and_arithmetic_signs_show_no_cell() {
    assert_eq!(cells_for("2015 · From"), "2015 FROM");
    assert_eq!(cells_for("an· other"), "AN OTHER");
    assert_eq!(cells_for("world:•spread"), "WORLD SPREAD");
    assert_eq!(cells_for("Dream×Vision"), "DREAM VISION");
    assert_eq!(cells_for("70 ÷ 7"), "70 7");
}

#[test]
fn a_space_with_a_combining_acute_accent_serves_as_an_apostrophe() {
    assert_eq!(cells_for("we don \u{301}t go"), "WE DONT GO");
}

#[test]
fn punctuation_at_the_ends_of_a_quote_adds_no_space() {
    assert_eq!(cells_for("“Hello there.”"), "HELLO THERE");
}

#[test]
fn apostrophes_brackets_and_some_symbols_stay_inside_a_word() {
    assert_eq!(cells_for("don't"), "DONT");
    // A curly apostrophe.
    assert_eq!(cells_for("don\u{2019}t"), "DONT");
    // An editor's bracket around a changed letter.
    assert_eq!(cells_for("[T]he"), "THE");
    // An asterisk censoring a letter.
    assert_eq!(cells_for("f*ck"), "FCK");
    assert_eq!(cells_for("AT&T"), "ATT");
    assert_eq!(cells_for("Ke$ha"), "KEHA");
}

#[test]
fn a_dot_between_single_letters_stays_inside_the_word() {
    assert_eq!(cells_for("U.S.A."), "USA");
}

#[test]
fn a_symbol_inside_a_number_shows_as_written() {
    assert_eq!(cells_for("1,000"), "1,000");
    assert_eq!(cells_for("3.5"), "3.5");
    // A Bible verse, or a clock time such as 5:30.
    assert_eq!(cells_for("John 3:16"), "JOHN 3:16");
    assert_eq!(cells_for("post-9/11"), "POST-9/11");
    assert_eq!(cells_for("10^9"), "10^9");
    assert_eq!(cells_for("2+2=5"), "2+2=5");
    assert_eq!(cells_for("24\\7"), "24\\7");
    // A curly apostrophe, as in feet and inches, shows in its ASCII form.
    assert_eq!(cells_for("5\u{2019}3"), "5'3");
}

#[test]
fn a_fraction_character_shows_as_digits_and_a_slash() {
    assert_eq!(cells_for("the 12 ½ years"), "THE 12 1/2 YEARS");
    assert_eq!(cells_for("a ¾ oz"), "A 3/4 OZ");
}

#[test]
fn a_fraction_character_after_a_digit_stays_apart_from_it() {
    assert_eq!(cells_for("Platform 9¾"), "PLATFORM 9 3/4");
}

#[test]
fn a_symbol_with_whitespace_beside_it_stays_outside_a_number() {
    assert_eq!(cells_for("1+1 = 2"), "1+1 2");
    assert_eq!(cells_for("1, 2"), "1 2");
}

#[test]
fn a_dash_shows_as_a_hyphen_rather_than_a_space() {
    // A hyphen, joining a compound word.
    assert_eq!(cells_for("well-known"), "WELL-KNOWN");
    // An en dash, joining the ends of a range.
    assert_eq!(cells_for("1990\u{2013}2000"), "1990-2000");
    // An em dash, setting off a break in thought.
    assert_eq!(cells_for("truly\u{2014}so"), "TRULY-SO");
    // A hyphen with a space on each side, standing in for a dash.
    assert_eq!(cells_for("truly - so"), "TRULY - SO");
}

#[test]
fn other_unicode_spaces_count_as_spaces() {
    // A thin space, an em space, a line separator, and an ideographic space.
    assert_eq!(
        cells_for("one\u{2009}two\u{2003}three\u{2028}four\u{3000}five"),
        "ONE TWO THREE FOUR FIVE"
    );
}

#[test]
fn a_listed_variant_of_a_mark_lays_out_as_the_mark() {
    // A horizontal bar and a minus sign, as dashes.
    assert_eq!(cells_for("did―it"), "DID-IT");
    assert_eq!(cells_for("world−view"), "WORLD-VIEW");
    // A modifier letter apostrophe and a modifier letter prime, as apostrophes.
    assert_eq!(cells_for("itʼs"), "ITS");
    assert_eq!(cells_for("hereʹs"), "HERES");
    // A reversed quotation mark, and Spanish inverted marks.
    assert_eq!(cells_for("‟Hi,” he said"), "HI HE SAID");
    assert_eq!(cells_for("¿Por qué? ¡Ya!"), "POR QUE YA");
}

#[test]
fn an_unlisted_mark_fails_even_when_its_ascii_form_is_listed() {
    // `any_ascii` writes these as `*` and `?`, which have cells.
    assert!(error_for("name ★FEYI★").contains("U+2605"));
    assert!(error_for("me \u{fffd}kite").contains("U+FFFD"));
}
