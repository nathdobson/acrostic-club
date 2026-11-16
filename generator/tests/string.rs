// Tests for `GraphemeString`'s debug form.

use acrostic_generator::string::GraphemeString;

#[test]
fn the_debug_form_shows_non_ascii_letters_beside_their_ascii_spelling() {
    let grapheme_string = GraphemeString::from_str("*straßé*");
    let debug_form = format!("{grapheme_string:?}");
    assert_eq!(debug_form, "*STRA[SS/ß][E/é]*");
}
