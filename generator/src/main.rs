#![allow(unused_imports, unused_variables, dead_code)]
#![feature(map_try_insert)]
#![feature(step_trait)]
#![feature(allocator_api)]
#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![feature(pattern)]
#![allow(unused_imports)]
#![feature(type_alias_impl_trait)]
#![feature(const_async_blocks)]
#![feature(try_blocks)]
#![feature(trait_alias)]
#![feature(error_generic_member_access)]
#![feature(impl_trait_in_assoc_type)]
#![feature(unboxed_closures)]
#![feature(arbitrary_self_types)]
#![feature(unwrap_infallible)]
#![feature(associated_type_defaults)]
#![feature(never_type)]
#![feature(ptr_metadata)]
#![feature(layout_for_ptr)]
extern crate core;

use futures::{stream, SinkExt};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::ErrorKind;
use std::ops::{Deref, Index, IndexMut};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{env, fs, io, mem};

use acrostic_core::letter::LetterSet;
use dict::build_dict;
use memmap::MmapOptions;
use ndarray::Array2;
use ndarray_npy::ReadNpyExt;
use npy::NpyData;
use npy_derive::Serializable;
use quote::build_quotes;
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_pickle::{DeOptions, HashableValue, Value};
use tikv_jemallocator::Jemalloc;
use tokio::select;
use tokio::signal::ctrl_c;
use trie::build_trie;

use crate::clues::{add_chat, ClueClient};
use crate::quote::add_quote;
use crate::search::add_answers;
// use crate::segment::add_letters;
use crate::site::build_site;
// use crate::turtle::build_turtle;

pub mod dict;
pub mod model;
pub mod puzzle;
pub mod search;
pub mod trie;
pub mod trie_table;
// pub mod segment;
mod add_letters;
mod banned;
pub mod clues;
pub mod llm;
pub mod ontology;
pub mod quote;
pub mod site;
pub mod string;
pub mod subseq;
pub mod turtle;
pub mod util;
mod lemma;
pub mod conflict_set;
mod cluedb;

use crate::stream::StreamExt;
use crate::util::interrupt::{CleanupSender, run_with_interrupts};
use add_letters::add_letters;
use crate::turtle::db::build_ontolex_turtle;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub static PACKAGE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::var("CARGO_MANIFEST_DIR")
        .map(|x| PathBuf::from(x).join(".."))
        .unwrap_or(PathBuf::from(env::current_dir().unwrap()));
    println!("PACKAGE_PATH = {:?}", path);
    path
});

pub async fn read_path(path: &Path) -> io::Result<Vec<u8>> {
    tokio::fs::read(path)
        .await
        .map_err(|e| io::Error::new(e.kind(), format!("Cannot read {:?}: {}", path, e)))
}

pub async fn read_path_to_string(path: &Path) -> io::Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| io::Error::new(e.kind(), format!("Cannot read {:?}: {}", path, e)))
}

pub async fn write_path(path: &Path, x: &[u8]) -> io::Result<()> {
    tokio::fs::write(path, x)
        .await
        .map_err(|e| io::Error::new(e.kind(), format!("Cannot write {:?}: {}", path, e)))
}

const QUOTE: &str = concat!(
    "And you've got to put your bodies upon the gears and ",
    "upon the wheels, upon the levers, ",
    "upon all the apparatus, and you've got to make it stop!",
    "...",
    "unless you're free the machine will be prevented from working at all",
);

const AUTHOR_TITLE: &str = concat!("Savio, Sprout Hall Address");

fn quote_letters() -> LetterSet {
    LetterSet::from_str(QUOTE)
}

fn author_title_letters() -> LetterSet {
    LetterSet::from_str(AUTHOR_TITLE)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(run_with_interrupts(main_impl).await?)
}

async fn main_impl(cleanup: CleanupSender) -> anyhow::Result<()> {
    let mut args = std::env::args().collect::<Vec<_>>().into_iter();
    args.next().unwrap();
    match args.next().as_deref() {
        Some("global") => match args.next().as_deref() {
            Some("quotes") => build_quotes().await?,
            Some("dict") => build_dict().await?,
            Some("trie") => build_trie().await?,
            Some("site") => build_site().await?,
            Some("turtle") => build_ontolex_turtle().await?,
            x => panic!("Unknown global target {:?}", x),
        },
        Some("puzzle") => {
            let target = args.next().unwrap();
            let mut puzzles = BTreeSet::new();
            for puzzle in args {
                if let Some((start, end)) = puzzle.split_once("-") {
                    for x in start.parse().unwrap()..=end.parse().unwrap() {
                        puzzles.insert(x);
                    }
                } else {
                    puzzles.insert(puzzle.parse().unwrap());
                }
            }
            let client = ClueClient::new(cleanup).await?;
            let concurrency = match target.deref() {
                "chat" => 1,
                _ => 1,
            };
            let errors = stream::iter(puzzles.into_iter())
                .map(|puzzle| {
                    let target = &target;
                    let client = &client;
                    async move {
                        let e: anyhow::Result<()> = try {
                            match target.deref() {
                                "quote" => add_quote(puzzle).await?,
                                "letters" => add_letters(puzzle).await?,
                                "answers" => add_answers(puzzle).await?,
                                "chat" => add_chat(puzzle, &client).await?,
                                x => panic!("Unknown puzzle target {}", x),
                            }
                        };
                        if let Err(e) = e {
                            if e.downcast_ref::<io::Error>()
                                .map_or(true, |x| x.kind() != io::ErrorKind::NotFound)
                            {
                                eprintln!("puzzle={} {}", puzzle, e);
                                return Some(e);
                            }
                        } else {
                            eprintln!("puzzle={} done", puzzle);
                        }
                        None
                    }
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<Option<anyhow::Error>>>()
                .await;
            mem::drop(client);
            eprintln!("{:?}", errors);
        }
        x => panic!("Unknown root command {:?}", x),
    }
    Ok(())
}
