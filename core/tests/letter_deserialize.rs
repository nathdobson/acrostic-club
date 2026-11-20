// Tests for reading a `Letter` through serde, which hands it the text of a JSON string such as
// `"q"`.

use acrostic_core::letter::Letter;
use serde::Deserialize;
use serde::de::IntoDeserializer;
use serde::de::value::{Error, StrDeserializer};

// Reads a letter from `text`, as a JSON string holding `text` would supply it.
fn deserialize_letter(text: &str) -> Result<Letter, Error> {
    let deserializer: StrDeserializer<Error> = text.into_deserializer();
    Letter::deserialize(deserializer)
}

#[test]
fn a_lowercase_letter_deserializes_as_that_letter() {
    let letter = deserialize_letter("q").expect("`q` should deserialize as a letter");
    assert_eq!(letter.to_char(), 'Q');
}

#[test]
fn an_uppercase_letter_deserializes_as_that_letter() {
    let letter = deserialize_letter("Q").expect("`Q` should deserialize as a letter");
    assert_eq!(letter.to_char(), 'Q');
}

#[test]
fn a_digit_fails_to_deserialize() {
    let error = deserialize_letter("7").expect_err("`7` should fail, since it isn't a letter");
    assert!(error.to_string().contains("expected a letter"), "{error}");
}

#[test]
fn a_letter_outside_ascii_fails_to_deserialize() {
    let error = deserialize_letter("é").expect_err("`é` should fail, since it isn't ASCII");
    assert!(error.to_string().contains("expected a letter"), "{error}");
}

#[test]
fn an_empty_string_fails_to_deserialize() {
    let error = deserialize_letter("").expect_err("an empty string should fail");
    assert!(error.to_string().contains("expected a letter"), "{error}");
}
