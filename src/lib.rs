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

extern crate core;

pub mod alloc;
pub mod flat_dict;
pub mod flat_trie;
pub mod flat_trie_table;
pub mod letter;
pub mod model;
pub mod search;
pub mod puzzle;
pub mod segment;
pub mod quote;

use tikv_jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use std::collections::HashMap;
use std::default::default;
use std::fmt::{Debug, Display, Formatter};
use std::{env, fs};
use std::fs::File;
use std::ops::{Index, IndexMut};
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

use crate::letter::{Letter, LetterMap, LetterSet};

pub static PACKAGE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or("".to_string())));

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

