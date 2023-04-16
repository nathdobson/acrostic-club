use std::collections::HashMap;
use std::io;

use itertools::Itertools;

use crate::alloc::MmapAllocator;
use crate::dict::{FLAT_WORDS, FlatWord};
use crate::trie::{FlatTrie, FlatTrieEntry};
use crate::{Letter, LetterMap, LetterSet, PACKAGE_PATH};

pub struct FlatTrieTable {
    pub dict: &'static [FlatWord],
    pub unary: LetterMap<Box<FlatTrie<LetterSet>, MmapAllocator>>,
    pub binary: HashMap<(Letter, Letter), Box<FlatTrie<(LetterSet, LetterSet)>, MmapAllocator>>,
}

impl FlatTrieTable {
    pub async fn new() -> io::Result<Self> {
        unsafe {
            let mut unary = LetterMap::new();
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
                    FlatTrie::restore(
                        &PACKAGE_PATH.join(&format!("build/binary/map_{}_{}.dat", l1, l2)),
                    ).await?,
                );
            }
            Ok(FlatTrieTable {
                dict: FLAT_WORDS.get().await?,
                unary,
                binary,
            })
        }
    }
}

#[test]
fn test_flat_trie_table() {
    let table = FlatTrieTable::new();
    for x in table.unary.iter() {
        println!("{:?}", x.0);
        format!("{:?}", x.1);
    }
    for x in table.binary.iter() {
        println!("{:?}", x.0);
        format!("{:?}", x.1);
    }
}
