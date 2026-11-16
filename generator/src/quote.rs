use std::io::{self, ErrorKind, Read};
use std::sync::LazyLock;
use std::time::Instant;

use csv::StringRecord;
use safe_once_async::async_lazy::AsyncLazy;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
use serde::Deserialize;
use serde::Serialize;

use crate::puzzle::Puzzle;
use crate::util::lazy_async::CloneError;
use crate::{PACKAGE_PATH, read_path, read_path_to_string, write_path};
// use crate::util::lazy_async::LazyAsync;

// use crate::util::lazy_async::LazyAsync;

#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    pub quote: String,
    pub source: String,
    pub topics: Vec<String>,
}

pub static QUOTES: LazyLock<AsyncLazyLock<JoinTransparent<io::Result<Vec<Quote>>>>> =
    LazyLock::new(|| {
        AsyncLazy::new(spawn_transparent(async {
            Ok(serde_json::from_str(
                &read_path_to_string(&PACKAGE_PATH.join("build/quotes.json")).await?,
            )?)
        }))
    });

// The records of the quote file that get no puzzle number.
//
// Puzzle numbers must stay stable: published links and solvers' saved progress refer to puzzles by
// number. The numbers in use came from an earlier hand-written parser, which merged a few groups of
// records into single entries and dropped one empty record. Each merged group's first record keeps
// its entry's number, and the records listed here get none, so every later puzzle number stays
// where that parser put it.
//
// The list fits the Quotes-500K submodule at commit b82f2b88. Another commit of the submodule may
// need a different list; `test_pinned_quote_file_matches_committed_puzzles` catches one that no
// longer fits.
#[rustfmt::skip]
const PINNED_UNNUMBERED_RECORDS: &[usize] = &[
    // Merged with record 29,312 by a backslash after this record's opening quotation mark.
    29_313,
    // Merged with record 57,766, because this record spreads its quote across several fields and
    // has an empty field before its topics.
    57_767,
    // Merged with record 301,314 by a backslash after record 301,317's opening quotation mark.
    301_315, 301_316, 301_317,
    // An empty record, which the earlier parser dropped.
    422_587,
];

impl Quote {
    // Builds a quote from one record of the quote file. A record holds the quote, its source, and
    // its comma-separated topics, followed by empty padding fields:
    //
    // - `Carpe diem.,Horace,"time, life",,,` gives the quote `Carpe diem.`, the source `Horace`,
    //   and the topics `time` and `life`.
    // - `Veni, vidi, vici.,Julius Caesar,history,,,` splits its quote across three fields. The
    //   last two fields are still the source and the topics, and the pieces before them rejoin,
    //   with commas, into the quote.
    // - `Carpe diem.,Horace,,,` has no topics, so its two remaining fields are the quote and the
    //   source.
    // - `Veni, vidi, vici.,Julius Caesar,,,` splits its quote and has no topics. The pieces after
    //   the first start with a space, so they rejoin the quote, and the last field is the source.
    // - `Carpe diem.,Horace, Odes,,,` splits its source and has no topics. ` Odes` starts with a
    //   space, so it rejoins the source, giving `Horace, Odes`.
    fn from_record(record: &StringRecord) -> Self {
        let mut fields: Vec<&str> = record.iter().collect();
        while fields.last() == Some(&"") {
            fields.pop();
        }
        // A field written without quotation marks splits at each of its commas. Where a space
        // followed a comma, the next piece starts with that space, as ` vici.` and ` Odes` do. A
        // whole field almost never starts with whitespace, so a field that does rejoins the field
        // before it. Rejoining may misread the records listed in `tasks.md #known-data-issues`.
        let mut rejoined_fields: Vec<String> = vec![];
        for field in fields {
            if field.starts_with(char::is_whitespace)
                && let Some(previous_field) = rejoined_fields.last_mut()
            {
                previous_field.push(',');
                previous_field.push_str(field);
            } else {
                rejoined_fields.push(field.to_string());
            }
        }
        // TODO: About two dozen records in the pinned file are misread: each holds an empty field
        // mid-record, such as one between the quote and the source.
        let (quote_pieces, source, topics) = match rejoined_fields.as_slice() {
            [quote, source] => (std::slice::from_ref(quote), source.as_str(), ""),
            // A quote split without a space after a comma still spans several fields here.
            [quote_pieces @ .., source, topics] => (quote_pieces, source.as_str(), topics.as_str()),
            // No fields, or a quote alone.
            quote_pieces => (quote_pieces, "", ""),
        };
        Self {
            quote: quote_pieces.join(","),
            source: source.to_string(),
            // TODO: A record without topics gets the topic list `[""]` rather than an empty list.
            topics: topics
                .split(",")
                .map(|topic| topic.trim().to_string())
                .collect(),
        }
    }
}

// Parses the quote file's CSV into a list indexed by puzzle number, giving every record except
// `unnumbered_records` the next number in turn. Fails when the input can't be read or isn't UTF-8,
// or when its records differ in width. Malformed CSV, such as an unclosed quotation mark, never
// fails on its own: the csv crate reads it leniently.
pub fn parse_quotes(input: impl Read, unnumbered_records: &[usize]) -> io::Result<Vec<Quote>> {
    // The csv crate reads the first record as a header unless told otherwise. This file has no
    // header row, and skipping its first record would shift every puzzle number.
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(input);
    let mut quotes = vec![];
    for (record_index, record) in reader.records().enumerate() {
        let record = record?;
        if unnumbered_records.contains(&record_index) {
            continue;
        }
        quotes.push(Quote::from_record(&record));
    }
    Ok(quotes)
}

// Parses the pinned Quotes-500K file, given its brotli-compressed bytes.
pub fn parse_pinned_quote_file(compressed: &[u8]) -> io::Result<Vec<Quote>> {
    parse_quotes(
        brotli::Decompressor::new(compressed, 4096),
        PINNED_UNNUMBERED_RECORDS,
    )
}

// Writes the pinned Quotes-500K file's quotes, indexed by puzzle number, to build/quotes.json.
pub async fn build_quotes() -> io::Result<()> {
    let start = Instant::now();
    let compressed = read_path(&PACKAGE_PATH.join("submodules/Quotes-500K/quotes.csv.br")).await?;
    let quotes = parse_pinned_quote_file(&compressed)?;
    write_path(
        &PACKAGE_PATH.join("build/quotes.json"),
        serde_json::to_string_pretty(&quotes)?.as_bytes(),
    )
    .await?;
    let elapsed = start.elapsed();
    println!("{elapsed:?}");
    Ok(())
}

pub async fn add_quote(pindex: usize) -> io::Result<()> {
    let quotes = QUOTES.get().await.clone_error_static()?;
    let quote = &quotes[pindex];
    if !(quote.source.len() >= 23
        && quote.source.len() <= 26
        && quote.quote.len() > 180
        && quote.quote.len() < 200)
    {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("bad quote {} {}", quote.source.len(), quote.quote.len()),
        ));
    }

    // let (index, selected) = quotes.into_iter().enumerate()
    //     .filter(|(index, quote)| quote.source.len() > 22
    //         && quote.source.len() <= 24
    //         && quote.quote.len() > 180
    //         && quote.quote.len() < 200).nth(pindex).unwrap();
    let puzzle = Puzzle {
        quote: quote.quote.clone(),
        quote_letters: None,
        source: quote.source.clone(),
        source_letters: None,
        clues: None,
        chat: None,
    };
    puzzle.write(pindex, "stage0.json").await?;
    Ok(())
}
