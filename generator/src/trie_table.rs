use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::sync::LazyLock;

use itertools::Itertools;
use safe_once_async::sync::AsyncStaticLock;
use acrostic_core::letter::{Letter, LetterMap, LetterSet};

use crate::dict::{FLAT_WORDS, FlatWord};
use crate::PACKAGE_PATH;
use crate::trie::{FlatTrie, FlatTrieEntry};
use crate::util::alloc::MmapAllocator;
use crate::util::lazy_async::CloneError;
use crate::util::persist::PersistentFile;
// use crate::util::lazy_async::LazyAsync;

// use crate::util::lazy_async::LazyAsync;
// use crate::util::lazy_async::LazyAsync;

pub struct FlatTrieTable {
    pub dict: &'static [FlatWord],
    pub unary: LetterMap<Box<FlatTrie<LetterSet>, MmapAllocator>>,
    pub binary: HashMap<(Letter, Letter), Box<FlatTrie<(LetterSet, LetterSet)>, MmapAllocator>>,
}

pub static FLAT_TRIE_TABLE: AsyncStaticLock<anyhow::Result<FlatTrieTable>> =
    AsyncStaticLock::new(async { FlatTrieTable::new().await });

impl FlatTrieTable {
    async fn new() -> anyhow::Result<Self> {
        unsafe {
            let mut unary: LetterMap<Option<Box<FlatTrie<LetterSet>, MmapAllocator>>> = LetterMap::new();
            for x in Letter::all() {
                unary[x] = Some(FlatTrie::restore(&PACKAGE_PATH.join(&format!("build/unary/map_{}.dat", x))).await?);
            }
            let unary = unary.map(|x| x.unwrap());
            let mut binary = HashMap::new();
            for ls in Letter::all().combinations_with_replacement(2) {
                let l1 = ls[0];
                let l2 = ls[1];
                binary.insert(
                    (l1, l2),
                    FlatTrie::<(LetterSet, LetterSet)>::restore(
                        &PACKAGE_PATH.join(&format!("build/binary/map_{}_{}.dat", l1, l2)),
                    ).await?,
                );
            }
            Ok(FlatTrieTable {
                dict: FLAT_WORDS.get_static().await?,
                unary,
                binary,
            })
        }
    }
}

// #[test]
// fn test_flat_trie_table() {
//     let table = FlatTrieTable::new();
//     for x in table.unary.iter() {
//         println!("{:?}", x.0);
//         format!("{:?}", x.1);
//     }
//     for x in table.binary.iter() {
//         println!("{:?}", x.0);
//         format!("{:?}", x.1);
//     }
// }
