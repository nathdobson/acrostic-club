#![allow(unused_imports, unused_variables, dead_code)]
#![deny(unused_must_use)]

use std::io::Cursor;
use std::time::Instant;
use std::{fs, io, slice};

use acrostic::PACKAGE_PATH;
use csv::{ReaderBuilder, StringRecord};
use itertools::{peek_nth, Itertools, PeekNth};
use serde::{Deserialize, Serialize};
use acrostic::quote::Quote;


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

#[tokio::main]
async fn main() -> io::Result<()> {
    // Retrieved from https://github.com/ShivaliGoel/Quotes-500K
    let mut contents = tokio::fs::read(PACKAGE_PATH.join("data/quotes.csv")).await?;
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
    tokio::fs::write(
        PACKAGE_PATH.join("data/quotes.json"),
        serde_json::to_string_pretty(&entries)?,
    )
    .await?;
    let start = Instant::now();

    println!("{:?}", start.elapsed());
    Ok(())
}
