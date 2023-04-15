use std::collections::HashMap;

use itertools::Itertools;

use crate::alloc::MmapAllocator;
use crate::flat_dict::FlatWord;
use crate::flat_trie::{FlatTrie, FlatTrieEntry};
use crate::{Letter, LetterMap, LetterSet, PACKAGE_PATH};

pub struct FlatTrieTable {
    pub dict: Box<[FlatWord], MmapAllocator>,
    pub unary: LetterMap<Box<FlatTrie<LetterSet>, MmapAllocator>>,
    pub binary: HashMap<(Letter, Letter), Box<FlatTrie<(LetterSet, LetterSet)>, MmapAllocator>>,
}

impl FlatTrieTable {
    pub fn new() -> Self {
        unsafe {
            FlatTrieTable {
                dict: FlatWord::get(),
                unary: Letter::all()
                    .map(|x| {
                        FlatTrie::restore(&PACKAGE_PATH.join(&format!("index/unary/map_{}.dat", x)))
                    })
                    .collect(),
                binary: Letter::all()
                    .combinations_with_replacement(2)
                    .map(|ls| {
                        let l1 = ls[0];
                        let l2 = ls[1];
                        (
                            (l1, l2),
                            FlatTrie::restore(
                                &PACKAGE_PATH.join(&format!("index/binary/map_{}_{}.dat", l1, l2)),
                            ),
                        )
                    })
                    .collect(),
            }
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
