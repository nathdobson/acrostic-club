use std::fs;

use acrostic_generator::PACKAGE_PATH;
use acrostic_generator::puzzle::Puzzle;
use acrostic_generator::quote::{Quote, parse_pinned_quote_file, parse_quotes};

// Parses `csv` with every record numbered by its index.
fn parse_numbered_by_index(csv: &str) -> Vec<Quote> {
    parse_quotes(csv.as_bytes(), &[]).expect("the test input should parse")
}

#[test]
fn test_doubled_quotation_marks_are_collapsed() {
    let quotes = parse_numbered_by_index(r#""She said ""go"".",Author,topic"#);

    assert_eq!(quotes[0].quote, r#"She said "go"."#);
}

#[test]
fn test_backslash_is_kept_as_written() {
    let quotes = parse_numbered_by_index(r#""\with hope",Author,topic"#);

    assert_eq!(quotes[0].quote, r"\with hope");
}

#[test]
fn test_source_with_a_doubled_mark_beside_a_comma_is_kept_whole() {
    let quotes = parse_numbered_by_index(r#"Quote,"""Nickname"", Surname",topic"#);

    assert_eq!(quotes[0].source, r#""Nickname", Surname"#);
}

#[test]
fn test_first_record_is_a_quote_not_a_header() {
    let quotes = parse_numbered_by_index("First,Author,topic\nSecond,Author,topic\n");

    assert_eq!(quotes[0].quote, "First");
}

#[test]
fn test_quote_split_across_fields_is_rejoined_with_commas() {
    let quotes = parse_numbered_by_index("One, two,three,Author,topic");

    assert_eq!(quotes[0].quote, "One, two,three");
}

#[test]
fn test_source_of_a_split_quote_is_the_second_to_last_field() {
    let quotes = parse_numbered_by_index("One, two,three,Author,topic");

    assert_eq!(quotes[0].source, "Author");
}

#[test]
fn test_split_quote_without_topics_is_rejoined_whole() {
    let quotes = parse_numbered_by_index("Veni, vidi, vici.,Julius Caesar,,,");

    assert_eq!(quotes[0].quote, "Veni, vidi, vici.");
}

#[test]
fn test_source_of_a_split_quote_without_topics_is_the_last_field() {
    let quotes = parse_numbered_by_index("Veni, vidi, vici.,Julius Caesar,,,");

    // Read by position, ` vici.` would be the source and `Julius Caesar` the topics. But ` vici.`
    // starts with a space, which marks it as a piece of the quote.
    assert_eq!(quotes[0].source, "Julius Caesar");
}

#[test]
fn test_split_quote_without_source_or_topics_is_rejoined_whole() {
    let quotes = parse_numbered_by_index("Read, explore, write, read,,,");

    assert_eq!(quotes[0].quote, "Read, explore, write, read");
    assert_eq!(quotes[0].source, "");
}

#[test]
fn test_source_split_at_a_comma_is_rejoined_whole() {
    let quotes = parse_numbered_by_index("Carpe diem.,Horace, Odes,,,");

    // Read by position, ` Odes` would be the topics.
    assert_eq!(quotes[0].source, "Horace, Odes");
}

#[test]
fn test_split_source_followed_by_topics_is_rejoined_whole() {
    let quotes = parse_numbered_by_index("Carpe diem.,Horace, Odes,time,,,");

    assert_eq!(quotes[0].source, "Horace, Odes");
}

#[test]
fn test_trailing_padding_fields_are_ignored() {
    let quotes = parse_numbered_by_index("Quote,Author,topic,,,,");

    // With the padding kept, the source would be the empty second-to-last field.
    assert_eq!(quotes[0].source, "Author");
}

#[test]
fn test_record_without_topics_has_a_quote_and_source() {
    let quotes = parse_numbered_by_index("Quote,Author,,,");

    // The empty topics field drops along with the padding, leaving two fields.
    assert_eq!(quotes[0].quote, "Quote");
    assert_eq!(quotes[0].source, "Author");
}

#[test]
fn test_record_with_an_empty_source_keeps_its_topics() {
    let quotes = parse_numbered_by_index("Quote,,topic");

    assert_eq!(quotes[0].source, "");
    assert_eq!(quotes[0].topics, vec!["topic"]);
}

#[test]
fn test_record_with_only_a_quote_has_an_empty_source() {
    let quotes = parse_numbered_by_index("Quote,,,,");

    assert_eq!(quotes[0].quote, "Quote");
    assert_eq!(quotes[0].source, "");
}

#[test]
fn test_topics_are_split_at_commas_and_trimmed() {
    let quotes = parse_numbered_by_index(r#"Quote,Author,"love, life""#);

    assert_eq!(quotes[0].topics, vec!["love", "life"]);
}

#[test]
fn test_unnumbered_records_are_skipped() {
    let quotes = parse_quotes(
        "Zero,A,t\nOne,A,t\nTwo,A,t\nThree,A,t\n".as_bytes(),
        &[1, 2],
    )
    .expect("the test input should parse");

    let quote_texts: Vec<&str> = quotes.iter().map(|quote| quote.quote.as_str()).collect();
    assert_eq!(quote_texts, ["Zero", "Three"]);
}

// Reads the real quote file and the committed puzzles, so this test needs the Quotes-500K
// submodule checked out.
#[test]
#[ignore = "slow: parses the whole Quotes-500K file"]
fn test_pinned_quote_file_matches_committed_puzzles() {
    let compressed = fs::read(PACKAGE_PATH.join("submodules/Quotes-500K/quotes.csv.br"))
        .expect("the Quotes-500K submodule should be checked out");

    let quotes = parse_pinned_quote_file(&compressed).expect("the pinned quote file should parse");

    let puzzle_directories =
        fs::read_dir(PACKAGE_PATH.join("puzzles")).expect("puzzles/ should exist");
    let mut checked_puzzle_count = 0;
    for puzzle_directory in puzzle_directories {
        let puzzle_directory = puzzle_directory
            .expect("puzzles/ should be readable")
            .path();
        // Skip stray files such as macOS's `.DS_Store`.
        if !puzzle_directory.is_dir() {
            continue;
        }
        let number: usize = puzzle_directory
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse().ok())
            .expect("each puzzle directory should be named by its number");
        let stage0 = fs::read_to_string(puzzle_directory.join("stage0.json"))
            .expect("each puzzle should have a stage0.json");
        let stored_puzzle: Puzzle =
            serde_json::from_str(&stage0).expect("stage0.json should parse");
        let quote = quotes
            .get(number)
            .unwrap_or_else(|| panic!("puzzle {number} should have a quote in the parsed list"));
        assert_eq!(quote.quote, stored_puzzle.quote, "puzzle {number}");
        assert_eq!(quote.source, stored_puzzle.source, "puzzle {number}");
        checked_puzzle_count += 1;
    }
    assert!(
        checked_puzzle_count > 0,
        "puzzles/ should hold at least one committed puzzle"
    );
}
