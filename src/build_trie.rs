#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::{BTreeMap, HashMap};
use std::{fs, io};

use tokio::main;
use crate::dict::FlatWord;
use crate::{Letter, LetterSet, PACKAGE_PATH};
use crate::alloc::save_vec;
use crate::flat_trie::{FlatTrie, FlatTrieEntry};

pub async fn build_trie() -> io::Result<()> {
    let dict = FlatWord::get().await?;
    let mut binary = BTreeMap::<(Letter, Letter), Vec<(LetterSet, (LetterSet, LetterSet))>>::new();
    let mut unary = BTreeMap::<Letter, Vec<(LetterSet, LetterSet)>>::new();
    for l in Letter::all() {
        unary.insert(l, vec![]);
    }
    for l1 in Letter::all() {
        for l2 in Letter::all() {
            binary.insert((l1, l2), vec![]);
        }
    }
    let mut words = vec![];
    for word in &*dict {
        if word.letters.count() > 5 {
            words.push(word);
        }
    }
    let words = &words[0..15000];
    for word1 in words {
        if let Some(first1) = word1.letter_vec.first() {
            unary
                .entry(*first1)
                .or_default()
                .push((word1.letters, word1.letters));
            for word2 in words {
                if let Some(first2) = word2.letter_vec.first() {
                    if first1 <= first2 {
                        binary.entry((*first1, *first2)).or_default().push((
                            word1.letters + word2.letters,
                            (word1.letters, word2.letters),
                        ));
                    }
                }
            }
        }
    }

    for (l1, vec) in unary {
        println!("{:?}", l1);
        save_vec::<FlatTrieEntry<LetterSet>>(
            &PACKAGE_PATH.join(&format!("build/unary/map_{}.dat", l1)),
            vec.into_iter()
                .collect::<Box<FlatTrie<LetterSet>>>()
                .as_slice(),
        ).await?;
    }
    for ((l1, l2), vec) in binary {
        println!("{:?}/{:?}", l1, l2);
        save_vec::<FlatTrieEntry<(LetterSet, LetterSet)>>(
            &PACKAGE_PATH.join(&format!("build/binary/map_{}_{}.dat", l1, l2)),
            vec.into_iter()
                .collect::<Box<FlatTrie<(LetterSet, LetterSet)>>>()
                .as_slice(),
        ).await?;
    }
    Ok(())
}
