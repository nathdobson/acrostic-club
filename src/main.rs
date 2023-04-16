#![allow(unused_imports, unused_variables, dead_code)]
#![feature(default_free_fn)]
#![feature(map_try_insert)]
#![feature(array_zip)]
#![feature(step_trait)]
#![feature(option_get_or_insert_default)]
#![feature(slice_group_by)]
#![feature(allocator_api)]
#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![feature(pattern)]
#![feature(lazy_cell)]
#![allow(unused_imports)]
#![feature(type_alias_impl_trait)]
#![feature(const_async_blocks)]
#![feature(try_blocks)]

extern crate core;

pub mod dict;
pub mod trie;
pub mod trie_table;
pub mod letter;
pub mod model;
pub mod search;
pub mod puzzle;
pub mod segment;
pub mod quote;
pub mod chat;
pub mod clues;
pub mod site;
pub mod util;

use tikv_jemallocator::Jemalloc;
use dict::build_dict;
use trie::build_trie;
use quote::build_quotes;
use crate::quote::add_quote;

use std::collections::HashMap;
use std::default::default;
use std::fmt::{Debug, Display, Formatter};
use std::{env, fs, io};
use std::fs::File;
use std::io::ErrorKind;
use std::ops::{Deref, Index, IndexMut};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use any_ascii::any_ascii;
use memmap::MmapOptions;
use ndarray::Array2;
use ndarray_npy::ReadNpyExt;
use npy::NpyData;
use npy_derive::Serializable;
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_pickle::{DeOptions, HashableValue, Value};
use crate::chat::add_chat;
use crate::clues::add_clues;

use crate::letter::{Letter, LetterMap, LetterSet};
use crate::search::add_answers;
use crate::segment::add_letters;
use crate::site::build_site;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub static PACKAGE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or("".to_string())));

pub async fn read_path(path: &Path) -> io::Result<Vec<u8>> {
    tokio::fs::read(path).await.map_err(|e| io::Error::new(e.kind(), format!("Cannot read {:?}: {}", path, e)))
}

pub async fn read_path_to_string(path: &Path) -> io::Result<String> {
    tokio::fs::read_to_string(path).await.map_err(|e| io::Error::new(e.kind(), format!("Cannot read {:?}: {}", path, e)))
}

pub async fn write_path(path: &Path, x: &[u8]) -> io::Result<()> {
    tokio::fs::write(path, x).await.map_err(|e| io::Error::new(e.kind(), format!("Cannot write {:?}: {}", path, e)))
}

const QUOTE: &str = concat!(
"And you've got to put your bodies upon the gears and ",
"upon the wheels, upon the levers, ",
"upon all the apparatus, and you've got to make it stop!",
"...",
"unless you're free the machine will be prevented from working at all",
);

const AUTHOR_TITLE: &str = concat!("Savio, Sprout Hall Address");

fn quote_letters() -> LetterSet { LetterSet::from_str(QUOTE) }

fn author_title_letters() -> LetterSet { LetterSet::from_str(AUTHOR_TITLE) }

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut args = std::env::args();
    args.next().unwrap();
    match args.next().as_deref() {
        Some("global") => match args.next().as_deref() {
            Some("quotes") => build_quotes().await?,
            Some("dict") => build_dict().await?,
            Some("trie") => build_trie().await?,
            Some("site") => build_site().await?,
            x => panic!("Unknown global target {:?}", x),
        }
        Some("puzzle") => {
            let target = args.next().unwrap();
            let mut errors: Vec<io::Error> = vec![];
            for puzzle in args {
                let puzzle: usize = puzzle.parse().unwrap();
                let e = try {
                    match target.deref() {
                        "quote" => add_quote(puzzle).await?,
                        "letters" => add_letters(puzzle).await?,
                        "answers" => add_answers(puzzle).await?,
                        "chat" => add_chat(puzzle).await?,
                        "clues" => add_clues(puzzle).await?,
                        x => panic!("Unknown puzzle target {}", x)
                    }
                };
                if let Err(e) = e {
                    eprintln!("puzzle={} {:?}", puzzle, e);
                    errors.push(e)
                }
            }
            eprintln!("{:?}", errors);
        }
        x => panic!("Unknown root command {:?}", x),
    }
    // match arg.as_ref() {
    //     "build_quotes" => build_quotes().await?,
    //     "build_dict" => build_dict().await?,
    //     "build_trie" => build_trie().await?,
    //     "add_quote" => add_quote::add_quote().await?,
    //     _ => return Err(io::Error::new(ErrorKind::NotFound, format!("no such command: {:?}", arg))),
    // }
    Ok(())
}

