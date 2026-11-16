use std::sync::LazyLock;

use crate::PACKAGE_PATH;
use crate::dict::{FLAT_WORDS, FlatWord};
use crate::trie::FlatTrie;
use crate::util::alloc::MmapAllocator;
use acrostic_core::letter::{Letter, LetterMap, LetterSet};
use safe_once_async::async_lazy::AsyncLazy;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
// use crate::util::lazy_async::LazyAsync;

// use crate::util::lazy_async::LazyAsync;
// use crate::util::lazy_async::LazyAsync;

pub struct FlatTrieTable {
    pub dict: &'static [FlatWord],
    pub unary: LetterMap<Box<FlatTrie<LetterSet>, MmapAllocator>>,
}

pub static FLAT_TRIE_TABLE: LazyLock<
    AsyncLazyLock<JoinTransparent<anyhow::Result<FlatTrieTable>>>,
> = LazyLock::new(|| AsyncLazy::new(spawn_transparent(FlatTrieTable::new())));
// AsyncStaticLock<anyhow::Result<FlatTrieTable>> =
//     AsyncStaticLock::new(async { FlatTrieTable::new().await });

impl FlatTrieTable {
    async fn new() -> anyhow::Result<Self> {
        unsafe {
            let mut unary: LetterMap<Option<Box<FlatTrie<LetterSet>, MmapAllocator>>> =
                LetterMap::new();
            for x in Letter::all() {
                unary[x] = Some(
                    FlatTrie::restore(&PACKAGE_PATH.join(&format!("build/unary/map_{}.dat", x)))
                        .await?,
                );
            }
            let unary = unary.map(|x| x.unwrap());
            Ok(FlatTrieTable {
                dict: FLAT_WORDS.get_static().await?,
                unary,
            })
        }
    }

    // Finds every pair of words in the single-word tries, the first starting with `first_letter`
    // and the second with `second_letter`, whose letters together are a subset of
    // `available_letters` and number at least `min_length`.
    pub fn search_word_pairs(
        &self,
        first_letter: Letter,
        second_letter: Letter,
        available_letters: LetterSet,
        min_length: usize,
        found: &mut Vec<(LetterSet, LetterSet)>,
    ) {
        search_word_pairs(
            &self.unary[first_letter],
            &self.unary[second_letter],
            available_letters,
            min_length,
            found,
        );
    }
}

// Finds every pair of words, the first from `first_trie` and the second from `second_trie`, whose
// letters together are a subset of `available_letters` and number at least `min_length`. Both
// tries should hold each word's letters as both its key and its value, as the single-word tries
// do. The search walks `first_trie` once, for every first word whose letters are a subset of
// `available_letters`. Then, for each first word, the search walks `second_trie` for the second
// words whose letters are a subset of the letters left over.
pub fn search_word_pairs(
    first_trie: &FlatTrie<LetterSet>,
    second_trie: &FlatTrie<LetterSet>,
    available_letters: LetterSet,
    min_length: usize,
    found: &mut Vec<(LetterSet, LetterSet)>,
) {
    let mut first_words = vec![];
    first_trie.search_subsets_with_min_length(available_letters, 0, &mut first_words);
    let mut second_words = vec![];
    for first_word in first_words {
        let letters_left = available_letters - first_word;
        let second_min_length = min_length.saturating_sub(first_word.count());
        second_trie.search_subsets_with_min_length(
            letters_left,
            second_min_length,
            &mut second_words,
        );
        found.extend(
            second_words
                .drain(..)
                .map(|second_word| (first_word, second_word)),
        );
    }
}
