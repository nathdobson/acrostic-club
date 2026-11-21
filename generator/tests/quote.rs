use std::fs;
use std::io;

use acrostic_generator::PACKAGE_PATH;
use acrostic_generator::puzzle::Puzzle;
use acrostic_generator::quote::{
    PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS, Quote, RepairedQuote, parse_pinned_quote_file,
    parse_quotes, start_puzzle,
};

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

    // Read by position, " vici." would be the source and "Julius Caesar" the topics. But " vici."
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

    // Read by position, " Odes" would be the topics.
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
fn test_topics_do_not_fill_an_empty_source() {
    let quotes = parse_numbered_by_index("Quote,,topic");

    // The last field is still read as the topics, not as the source.
    assert_eq!(quotes[0].source, "");
}

#[test]
fn test_record_with_only_a_quote_has_an_empty_source() {
    let quotes = parse_numbered_by_index("Quote,,,,");

    assert_eq!(quotes[0].quote, "Quote");
    assert_eq!(quotes[0].source, "");
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

// A puzzle number off the allowlist, so that the length limits apply to it in full.
const UNALLOWLISTED_PUZZLE_NUMBER: usize = 1;

// Repairs `quote` and starts a puzzle from it under a number off the allowlist, as for a new
// puzzle.
fn start_unallowlisted_puzzle(quote: &Quote) -> io::Result<Puzzle> {
    start_puzzle(UNALLOWLISTED_PUZZLE_NUMBER, &RepairedQuote::new(quote))
}

// Builds a quote from `text` and `source` that fits stage 0's length limits, so that only their
// letters decide whether the quote starts a puzzle. Pads each with a space and x's. Neither input
// may hold an X, and the quote's padding holds more x's than the source's, so the padding never
// decides a test.
fn build_quote(text: &str, source: &str) -> Quote {
    for input in [text, source] {
        assert!(
            !input.contains(['x', 'X']),
            "a test input should hold no X, since the padding supplies x's: {input:?}"
        );
    }
    let pad = |unpadded: &str, letter_count: usize| {
        let unpadded_letter_count = unpadded
            .chars()
            .filter(|character| character.is_alphabetic())
            .count();
        let padding = "x".repeat(letter_count - unpadded_letter_count);
        format!("{unpadded} {padding}")
    };
    Quote {
        quote: pad(text, 175),
        source: pad(source, 24),
    }
}

#[test]
fn test_quote_holding_every_source_letter_starts_a_puzzle() {
    let quote = build_quote("Lola sat alone.", "Al Stone");

    start_unallowlisted_puzzle(&quote)
        .expect("a quote holding every letter of its source should start a puzzle");
}

#[test]
fn test_quote_lacking_a_source_letter_is_rejected() {
    // The source needs a Z, and the quote has none.
    let quote = build_quote("A rose is a rose.", "Ezra Rose");

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote without a Z should be rejected");

    assert!(error.to_string().contains("lacks: Z"), "{error}");
}

#[test]
fn test_quote_holding_too_few_of_a_source_letter_is_rejected() {
    // The source needs two N's, and the quote has one.
    let quote = build_quote("Ana sat.", "Anna");

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote with one N should be rejected");

    assert!(error.to_string().contains("lacks: N"), "{error}");
}

#[test]
fn test_quote_with_a_mark_that_has_no_cells_is_rejected() {
    let quote = build_quote("Healed ☥ at last.", "Al Hatt");

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote holding `☥` should be rejected");

    assert!(error.to_string().contains("U+2625"), "{error}");
}

#[test]
fn test_started_puzzle_holds_its_quote_with_characters_repaired() {
    let quote = build_quote("Lola sat a\u{ad}lone.", "Al Stone");

    let puzzle = start_unallowlisted_puzzle(&quote).expect("the quote should start a puzzle");

    let reformatted_quote = &puzzle.quote;
    assert!(
        reformatted_quote.starts_with("Lola sat alone. "),
        "{reformatted_quote:?}"
    );
}

#[test]
fn test_started_puzzle_holds_its_quote_with_lost_spaces_restored() {
    let quote = build_quote("Lola sat.Alone", "Al Stone");

    let puzzle = start_unallowlisted_puzzle(&quote).expect("the quote should start a puzzle");

    let reformatted_quote = &puzzle.quote;
    assert!(
        reformatted_quote.starts_with("Lola sat. Alone "),
        "{reformatted_quote:?}"
    );
}

#[test]
fn test_started_puzzle_repairs_characters_before_restoring_lost_spaces() {
    // The zero-width space hides the lost space after "sat." until the repair removes it.
    let quote = build_quote("Lola sat.\u{200b}Alone", "Al Stone");

    let puzzle = start_unallowlisted_puzzle(&quote).expect("the quote should start a puzzle");

    let reformatted_quote = &puzzle.quote;
    assert!(
        reformatted_quote.starts_with("Lola sat. Alone "),
        "{reformatted_quote:?}"
    );
}

#[test]
fn test_started_puzzle_holds_its_source_with_characters_repaired() {
    // "Ã©" is `é` misread from UTF-8 as Windows-1252.
    let quote = build_quote("Lola sat alone.", "Al StonÃ©");

    let puzzle = start_unallowlisted_puzzle(&quote).expect("the quote should start a puzzle");

    let repaired_source = &puzzle.source;
    assert!(
        repaired_source.starts_with("Al Stoné "),
        "{repaired_source:?}"
    );
}

#[test]
fn test_accented_quote_letter_supplies_its_plain_letter() {
    // The source's E comes from the quote's `é`.
    let quote = build_quote("Café", "Ace");

    start_unallowlisted_puzzle(&quote).expect("the quote's `é` should supply the source's E");
}

// Reads the real quote file and the committed puzzles, so this test needs the Quotes-500K submodule
// checked out.
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
        // Stage 0 stores the quote and source reformatted.
        let reformatted_quote = RepairedQuote::new(quote)
            .reformat()
            .unwrap_or_else(|error| panic!("puzzle {number}'s quote should lay out: {error}"));
        assert_eq!(
            reformatted_quote.quote, stored_puzzle.quote,
            "puzzle {number}"
        );
        assert_eq!(
            reformatted_quote.source, stored_puzzle.source,
            "puzzle {number}"
        );
        checked_puzzle_count += 1;
    }
    assert!(
        checked_puzzle_count > 0,
        "puzzles/ should hold at least one committed puzzle"
    );
}

#[test]
fn test_quote_and_source_at_their_lower_length_limits_start_a_puzzle() {
    let quote = Quote {
        quote: "a".repeat(166),
        source: "a".repeat(20),
    };

    start_unallowlisted_puzzle(&quote)
        .expect("a quote of 166 letters and a source of 20 should start a puzzle");
}

#[test]
fn test_quote_and_source_at_their_upper_length_limits_start_a_puzzle() {
    let quote = Quote {
        quote: "a".repeat(183),
        source: "a".repeat(26),
    };

    start_unallowlisted_puzzle(&quote)
        .expect("a quote of 183 letters and a source of 26 should start a puzzle");
}

#[test]
fn test_quote_below_its_length_limit_is_rejected() {
    let quote = Quote {
        quote: "a".repeat(165),
        source: "a".repeat(21),
    };

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote of 165 letters should be rejected");

    assert!(error.to_string().contains("165 letters"), "{error}");
}

#[test]
fn test_quote_above_its_length_limit_is_rejected() {
    let quote = Quote {
        quote: "a".repeat(184),
        source: "a".repeat(21),
    };

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote of 184 letters should be rejected");

    assert!(error.to_string().contains("184 letters"), "{error}");
}

#[test]
fn test_quote_holding_more_than_255_of_one_letter_is_rejected_for_its_length() {
    // 256 A's overflow a count that holds at most 255.
    let quote = Quote {
        quote: "a".repeat(256),
        source: "a".repeat(21),
    };

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote of 256 letters should be rejected");

    assert!(error.to_string().contains("256 letters"), "{error}");
}

#[test]
fn test_source_below_its_length_limit_is_rejected() {
    let quote = Quote {
        quote: "a".repeat(170),
        source: "a".repeat(19),
    };

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a source of 19 letters should be rejected");

    assert!(error.to_string().contains("source 19"), "{error}");
}

#[test]
fn test_source_above_its_length_limit_is_rejected() {
    let quote = Quote {
        quote: "a".repeat(183),
        source: "a".repeat(27),
    };

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a source of 27 letters should be rejected");

    assert!(error.to_string().contains("source 27"), "{error}");
}

#[test]
fn test_spaces_punctuation_and_digits_do_not_count_toward_length() {
    // 166 letters, the lower limit, among 664 characters, and a source of 20 letters among 60.
    let quote = Quote {
        quote: "a1, ".repeat(166),
        source: "a. ".repeat(20),
    };

    start_unallowlisted_puzzle(&quote).expect("a quote of 166 letters should start a puzzle");
}

#[test]
fn test_accented_letter_counts_as_one_letter() {
    // 183 letters, the upper limit, written in 366 bytes.
    let quote = Quote {
        quote: "é".repeat(183),
        source: "e".repeat(20),
    };

    start_unallowlisted_puzzle(&quote)
        .expect("a quote of 183 accented letters should start a puzzle");
}

#[test]
fn test_quote_with_too_short_an_average_answer_is_rejected() {
    // 6.38 letters per answer, though each length alone falls within its limits.
    let quote = Quote {
        quote: "a".repeat(166),
        source: "a".repeat(26),
    };

    let error = start_unallowlisted_puzzle(&quote)
        .expect_err("answers averaging 6.38 letters should be rejected");

    assert!(error.to_string().contains("averaging"), "{error}");
}

#[test]
fn test_quote_with_exactly_the_minimum_average_answer_length_starts_a_puzzle() {
    // 6.5 letters per answer.
    let quote = Quote {
        quote: "a".repeat(169),
        source: "a".repeat(26),
    };

    start_unallowlisted_puzzle(&quote)
        .expect("answers averaging 6.5 letters should start a puzzle");
}

#[test]
fn test_allowlisted_published_puzzle_outside_the_length_limits_starts() {
    let allowlisted_puzzle_number = PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS[0];
    let quote = Quote {
        quote: "a".repeat(10),
        source: "a".repeat(5),
    };

    start_puzzle(allowlisted_puzzle_number, &RepairedQuote::new(&quote))
        .expect("an allowlisted published puzzle should start whatever its length");
}

#[test]
fn test_unallowlisted_puzzle_outside_the_length_limits_is_rejected() {
    let quote = Quote {
        quote: "a".repeat(10),
        source: "a".repeat(5),
    };

    let error =
        start_unallowlisted_puzzle(&quote).expect_err("a quote of 10 letters should be rejected");

    assert!(error.to_string().contains("10 letters"), "{error}");
}

// Lists the numbers of the committed puzzles, which name the directories under `puzzles/`.
fn committed_puzzle_numbers() -> Vec<usize> {
    let puzzle_directories =
        fs::read_dir(PACKAGE_PATH.join("puzzles")).expect("puzzles/ should exist");
    puzzle_directories
        .map(|entry| entry.expect("puzzles/ should be readable").path())
        // Skip stray files such as macOS's `.DS_Store`.
        .filter(|path| path.is_dir())
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse().ok())
                .expect("each puzzle directory should be named by its number")
        })
        .collect()
}

// Whether a committed puzzle is published, which it is once it has the `stage3.json` that the site
// serves.
fn is_published(puzzle_number: usize) -> bool {
    PACKAGE_PATH
        .join(format!("puzzles/{puzzle_number}/stage3.json"))
        .exists()
}

// Reads the quote and source stored in a committed puzzle's `stage0.json`, which
// `test_pinned_quote_file_matches_committed_puzzles` checks against the quote file.
fn read_committed_quote(puzzle_number: usize) -> Quote {
    let path = PACKAGE_PATH.join(format!("puzzles/{puzzle_number}/stage0.json"));
    let stage0 = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{path:?} should be readable: {error}"));
    let puzzle: Puzzle = serde_json::from_str(&stage0)
        .unwrap_or_else(|error| panic!("{path:?} should parse: {error}"));
    Quote {
        quote: puzzle.quote,
        source: puzzle.source,
    }
}

#[test]
fn test_published_puzzles_start() {
    let published_puzzle_numbers: Vec<usize> = committed_puzzle_numbers()
        .into_iter()
        .filter(|&puzzle_number| is_published(puzzle_number))
        .collect();

    let start_failures: Vec<String> = published_puzzle_numbers
        .iter()
        .filter_map(|&puzzle_number| {
            let quote = RepairedQuote::new(&read_committed_quote(puzzle_number));
            start_puzzle(puzzle_number, &quote)
                .err()
                .map(|error| format!("puzzle {puzzle_number}: {error}"))
        })
        .collect();

    assert!(
        !published_puzzle_numbers.is_empty(),
        "puzzles/ should hold at least one published puzzle"
    );
    assert_eq!(
        start_failures,
        Vec::<String>::new(),
        "each published puzzle should start, so that rebuilding reproduces it"
    );
}

#[test]
fn test_allowlisted_published_puzzles_have_a_stage3() {
    let unpublished_puzzle_numbers: Vec<usize> = PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS
        .iter()
        .copied()
        .filter(|&puzzle_number| !is_published(puzzle_number))
        .collect();

    assert_eq!(unpublished_puzzle_numbers, Vec::<usize>::new());
}

#[test]
fn test_allowlisted_published_puzzles_would_not_start_without_the_allowlist() {
    let needlessly_allowlisted_puzzle_numbers: Vec<usize> = PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS
        .iter()
        .copied()
        .filter(|&puzzle_number| {
            start_unallowlisted_puzzle(&read_committed_quote(puzzle_number)).is_ok()
        })
        .collect();

    assert_eq!(needlessly_allowlisted_puzzle_numbers, Vec::<usize>::new());
}
