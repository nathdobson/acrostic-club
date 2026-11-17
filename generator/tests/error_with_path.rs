use std::io;
use std::path::Path;

use acrostic_generator::error_with_path;

#[test]
fn test_error_names_the_action_and_path() {
    let error = io::Error::from(io::ErrorKind::NotFound);

    let message = error_with_path("read", Path::new("build/turtle.dat"))(error).to_string();

    assert!(message.contains("read"), "{message}");
    assert!(message.contains("build/turtle.dat"), "{message}");
}

#[test]
fn test_error_keeps_its_kind() {
    let error = io::Error::from(io::ErrorKind::NotFound);

    let named_error = error_with_path("remove", Path::new("build/site"))(error);

    assert_eq!(named_error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn test_parse_error_names_the_path() {
    let parse_error = serde_json::from_str::<u32>("five").expect_err("a word should not parse");

    let message = error_with_path("parse", Path::new("build/quotes.json"))(parse_error).to_string();

    assert!(message.contains("build/quotes.json"), "{message}");
}
