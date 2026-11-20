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
fn unreformatted_punctuation_between_words_joins_them() {
    // Such text should never reach the cells, since stage 0 restores the space first. The test
    // pins down how the cells handle it anyway, not a result to rely on.
    assert_eq!(cells_for("end.Begin"), "ENDBEGIN");
}

#[test]
fn a_slash_between_words_shows_as_a_space() {
    assert_eq!(cells_for("and/or"), "AND OR");
    assert_eq!(cells_for("this / that"), "THIS THAT");
}

#[test]
fn tabs_newlines_and_non_breaking_spaces_count_as_spaces() {
    assert_eq!(cells_for("one\ttwo"), "ONE TWO");
    assert_eq!(cells_for("one\ntwo"), "ONE TWO");
    assert_eq!(cells_for("one\u{a0}two"), "ONE TWO");
}

#[test]
fn several_spaces_in_a_row_show_as_one() {
    assert_eq!(cells_for("one  two"), "ONE TWO");
}

#[test]
fn an_unrepaired_soft_hyphen_fails() {
    // Such text should never reach the cells, since stage 0 removes soft hyphens first. The test
    // pins down how the cells handle it anyway, not a result to rely on.
    let message = error_for("beauti\u{ad}ful");

    assert!(message.contains("U+00AD"), "{message}");
}

#[test]
fn vertical_tabs_and_form_feeds_count_as_spaces() {
    assert_eq!(cells_for("one\u{b}two\u{c}three"), "ONE TWO THREE");
}

#[test]
fn a_backtick_or_brace_stays_inside_a_word() {
    assert_eq!(cells_for("you`ll"), "YOULL");
    assert_eq!(cells_for("{T}he"), "THE");
}

#[test]
fn operators_and_separators_between_words_show_no_cell() {
    assert_eq!(cells_for("You + Me"), "YOU ME");
    assert_eq!(cells_for("school= pranks"), "SCHOOL PRANKS");
    assert_eq!(cells_for("LAWS> LAWS"), "LAWS LAWS");
    assert_eq!(cells_for("God+ Wisdom"), "GOD WISDOM");
    assert_eq!(cells_for("never <3"), "NEVER 3");
    assert_eq!(cells_for("world | the"), "WORLD THE");
    assert_eq!(cells_for("fire\\ How"), "FIRE HOW");
    assert_eq!(cells_for("@bobby i'm"), "BOBBY IM");
    assert_eq!(cells_for("so wrong!^^^ so"), "SO WRONG SO");
}

#[test]
fn other_languages_quotation_marks_show_no_cell() {
    assert_eq!(cells_for("«idealist»"), "IDEALIST");
    assert_eq!(cells_for("„Do we"), "DO WE");
}

#[test]
fn separator_dots_bullets_and_arithmetic_signs_show_no_cell() {
    assert_eq!(cells_for("2015 · From"), "2015 FROM");
    assert_eq!(cells_for("an· other"), "AN OTHER");
    assert_eq!(cells_for("world:• spread"), "WORLD SPREAD");
    assert_eq!(cells_for("Dream× Vision"), "DREAM VISION");
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
fn whitespace_at_the_ends_of_a_quote_shows_no_cell() {
    assert_eq!(cells_for(" Hello there. "), "HELLO THERE");
    // A mark and a space before the first word, and a space before the last mark.
    assert_eq!(cells_for("... I learned"), "I LEARNED");
    assert_eq!(cells_for("Is that so ?"), "IS THAT SO");
}

#[test]
fn dashes_at_the_ends_of_a_quote_show_no_cell() {
    assert_eq!(cells_for("- I learned"), "I LEARNED");
    assert_eq!(cells_for("I will do it -"), "I WILL DO IT");
    assert_eq!(cells_for("of attachment.-- "), "OF ATTACHMENT");
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
fn a_symbol_with_whitespace_beside_it_stays_outside_a_number() {
    assert_eq!(cells_for("1+1 = 2"), "1+1 2");
    assert_eq!(cells_for("1, 2"), "1 2");
}

#[test]
fn a_number_symbol_touching_a_number_shows_as_written() {
    assert_eq!(cells_for("99% of"), "99% OF");
    assert_eq!(cells_for("a salary of $200,000"), "A SALARY OF $200,000");
    assert_eq!(cells_for("the #1 thing"), "THE #1 THING");
    assert_eq!(cells_for("6+ years"), "6+ YEARS");
}

#[test]
fn a_number_symbol_on_the_unusual_side_of_a_number_shows_too() {
    assert_eq!(cells_for("%100 sure"), "%100 SURE");
    assert_eq!(cells_for("bet 200$ that"), "BET 200$ THAT");
}

#[test]
fn several_number_symbols_touching_a_number_all_show() {
    assert_eq!(cells_for("85%+ of"), "85%+ OF");
}

#[test]
fn a_number_symbol_after_a_range_shows_along_with_the_range() {
    // The first `%` sits inside the number `50%-60`, and the second touches its end.
    assert_eq!(cells_for("50%-60% of"), "50%-60% OF");
}

#[test]
fn a_number_symbol_at_either_end_of_a_quote_shows() {
    assert_eq!(cells_for("$13 to $20"), "$13 TO $20");
    assert_eq!(cells_for("give it 100%."), "GIVE IT 100%");
}

#[test]
fn a_number_symbol_spaced_apart_from_a_number_shows_no_cell() {
    assert_eq!(cells_for("99 % of"), "99 OF");
    assert_eq!(cells_for("the # 1 rule"), "THE 1 RULE");
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
