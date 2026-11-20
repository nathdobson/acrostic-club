// Tests for `split_into_runs`: which characters it rejects, since no cells are defined for them.

use acrostic_generator::words_and_marks::split_into_runs;

// The error message for a quote that should fail to split into runs.
fn error_for(quote: &str) -> String {
    match split_into_runs(quote) {
        Ok(_) => panic!("the quote {quote:?} should fail to split into runs"),
        Err(error) => error.to_string(),
    }
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
fn a_dingbat_with_the_ascii_form_of_a_known_symbol_fails() {
    // `any_ascii` writes these as `+`, `+`, `>`, and `=`, which have cells.
    assert!(error_for("healed ☥ She").contains("U+2625"));
    assert!(error_for("prince †1183").contains("U+2020"));
    assert!(error_for("LAWS→LAWS").contains("U+2192"));
    assert!(error_for("a ≡ b").contains("U+2261"));
}

#[test]
fn an_unlisted_mark_fails_even_when_its_ascii_form_is_listed() {
    // `any_ascii` writes these as `*` and `?`, which have cells.
    assert!(error_for("name ★FEYI★").contains("U+2605"));
    assert!(error_for("me \u{fffd}kite").contains("U+FFFD"));
}
