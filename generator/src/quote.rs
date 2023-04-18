use std::{fs, io, slice};
use std::io::{Cursor, ErrorKind, Read};
use std::sync::LazyLock;
use std::time::Instant;

use itertools::{peek_nth, PeekNth};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use serde::Serialize;

use crate::{PACKAGE_PATH, read_path, read_path_to_string, write_path};
use crate::puzzle::Puzzle;
use crate::util::lazy_async::LazyAsync;

// use crate::util::lazy_async::LazyAsync;

#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    pub quote: String,
    pub source: String,
    pub topics: Vec<String>,
}

pub static QUOTES: LazyLock<LazyAsync<io::Result<Vec<Quote>>>> = LazyLock::new(|| {
    LazyAsync::new(async {
        Ok(serde_json::from_str(&read_path_to_string(&PACKAGE_PATH.join("build/quotes.json")).await?)?)
    })
});

struct Parser<'a>(PeekNth<slice::Iter<'a, u8>>);

impl<'a> Parser<'a> {
    fn read_exact(&mut self, expected: &[u8]) -> bool {
        for (i, x) in expected.iter().enumerate() {
            if self.0.peek_nth(i) != Some(&x) {
                return false;
            }
        }
        self.0.nth(expected.len() - 1);
        true
    }
    fn read_exact_end(&mut self, expected: &[u8]) -> bool {
        if self.0.peek_nth(expected.len()).is_some() {
            return false;
        }
        self.read_exact(expected)
    }
    fn eof(&mut self) -> bool { self.0.peek().is_none() }
    fn read(&mut self) -> Option<u8> { self.0.next().copied() }
}

pub async fn build_quotes() -> io::Result<()> {
    let mut contents = read_path(&PACKAGE_PATH.join("submodules/Quotes-500K/quotes.csv.br")).await?;
    let mut dec = brotli::Decompressor::new(Cursor::new(&contents), 4096);
    let mut c2 = vec![];
    dec.read_to_end(&mut c2).unwrap();
    let mut contents = c2;
    contents.reverse();
    let mut parser = Parser(peek_nth(contents.iter()));
    let mut entries = vec![];
    'entries: loop {
        while parser.read_exact(b",") {}
        let mut cells = vec![];
        'cells: loop {
            let quoted = parser.read_exact(b"\"");
            let mut cell = vec![];
            loop {
                if parser.eof() {
                    cells.push(cell);
                    entries.push(cells);
                    break 'entries;
                }
                if parser.read_exact(b"\\") {
                    cell.push(parser.read().unwrap());
                    continue;
                }
                if quoted {
                    if cells.len() < 2 && parser.read_exact(b"\",") {
                        break;
                    }
                    if parser.read_exact(b"\"\n") {
                        cells.push(cell);
                        break 'cells;
                    }
                } else {
                    if parser.read_exact(b"\n") {
                        cells.push(cell);
                        break 'cells;
                    }
                    if cells.len() < 2 && parser.read_exact(b",") {
                        break;
                    }
                }
                if let Some(next) = parser.read() {
                    cell.push(next);
                    continue;
                } else {
                    break;
                }
            }
            cells.push(cell);
        }
        entries.push(cells);
    }
    entries.reverse();
    let mut entries: Vec<Vec<String>> = entries
        .into_iter()
        .map(|mut x| {
            x.reverse();
            let mut x: Vec<String> = x
                .into_iter()
                .map(|mut x| {
                    x.reverse();
                    String::from_utf8(x).unwrap()
                })
                .collect();
            x.resize(3, String::new());
            x
        })
        .collect();
    entries.retain(|x| x.iter().any(|x| !x.is_empty()));
    let entries: Vec<Quote> = entries
        .into_iter()
        .map(|x| Quote {
            quote: x[0].clone(),
            source: x[1].clone(),
            topics: x[2].split(",").map(|x| x.trim().to_string()).collect(),
        })
        .collect();
    write_path(
        &PACKAGE_PATH.join("build/quotes.json"),
        serde_json::to_string_pretty(&entries)?.as_bytes(),
    )
        .await?;
    let start = Instant::now();

    println!("{:?}", start.elapsed());
    Ok(())
}


pub async fn add_quote(pindex: usize) -> io::Result<()> {
    let mut quotes = QUOTES.get_io().await?;
    let quote = &quotes[pindex];
    if !(quote.source.len() > 24
        && quote.source.len() <= 26
        && quote.quote.len() > 180
        && quote.quote.len() < 200) {
        return Err(io::Error::new(ErrorKind::InvalidInput, "bad quote"));
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
