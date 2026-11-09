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

use futures::{SinkExt, stream};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::ErrorKind;
use std::ops::{Deref, Index, IndexMut};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{env, fs, io, mem};

use acrostic_core::letter::LetterSet;
use memmap::MmapOptions;
use ndarray::Array2;
use ndarray_npy::ReadNpyExt;
use npy::NpyData;
use npy_derive::Serializable;
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_pickle::{DeOptions, HashableValue, Value};
use tokio::select;
use tokio::signal::ctrl_c;

pub mod dict;
pub mod model;
pub mod puzzle;
pub mod search;
pub mod trie;
pub mod trie_table;
// pub mod segment;
pub mod add_letters;
mod banned;
mod cluedb;
pub mod clues;
pub mod conflict_set;
mod lemma;
pub mod llm;
pub mod ontology;
pub mod quote;
pub mod site;
pub mod string;
pub mod subseq;
pub mod turtle;
pub mod util;

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
