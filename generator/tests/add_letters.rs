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
fn a_comma_dot_or_colon_inside_a_number_stays_inside_it() {
    assert_eq!(cells_for("1,000"), "1000");
    assert_eq!(cells_for("3.5"), "35");
    // A Bible verse, or a clock time such as 5:30.
    assert_eq!(cells_for("John 3:16"), "JOHN 316");
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
