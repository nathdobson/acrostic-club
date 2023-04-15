#![allow(unused_imports)]
#![allow(unused_variables)]
#![deny(unused_must_use)]

use std::fmt::Write;
use std::fs;

use acrostic::letter::Letter;
use acrostic::puzzle::Puzzle;
use acrostic::segment::segment;
use acrostic::PACKAGE_PATH;
use itertools::EitherOrBoth;
use tokio::io;

fn get_letters(input: &str) -> String {
    let mut cells = String::new();
    for x in segment(&input) {
        if let Some(letter) = x.as_ref().left() {
            write!(&mut cells, "{}", letter).unwrap();
        } else if let Some(content) = x.right() {
            if content.chars().all(|x|x.is_numeric()){
                // write!(&mut cells, "{}",content).unwrap();
            }
            match &*content {
                " " => {
                    write!(&mut cells, "{}", content).unwrap();
                }
                "." | "," | ";" | "'" | "\"" | "!" | "?" | "‘" | "’" | ":" | "&" | "*" => {}
                "-" => {
                    write!(&mut cells, "-").unwrap();
                }
                x => todo!("{:?}", x),
            }
        }
    }
    cells
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut puzzle = Puzzle::read("stage0.json")?;
    puzzle
        .quote_letters
        .get_or_insert_with(|| get_letters(&puzzle.quote));
    puzzle.source_letters.get_or_insert_with(|| {
        get_letters(&puzzle.source)
            .chars()
            .filter(|x| x.is_ascii_alphabetic())
            .collect()
    });
    puzzle.write("stage1.json")?;
    Ok(())
}
