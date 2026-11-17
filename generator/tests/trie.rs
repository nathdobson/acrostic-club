// Tests for `FlatTrie`.

use acrostic_core::letter::LetterSet;
use acrostic_generator::trie::FlatTrie;

#[test]
fn a_flat_trie_builds_from_three_words() {
    let words = vec!["ab", "abc", "abd"];

    let trie: Box<FlatTrie<&str>> = words
        .into_iter()
        .map(|word| (LetterSet::from_str(word), word))
        .collect();

    let trie_entries = trie.as_slice();
    println!("{trie_entries:?}");
    println!("{trie:?}");
}

// Builds a trie whose keys are the letters of `words` and whose values are the words themselves.
fn build_word_trie(words: &[&'static str]) -> Box<FlatTrie<&'static str>> {
    words
        .iter()
        .map(|word| (LetterSet::from_str(word), *word))
        .collect()
}

// Returns the words `FlatTrie::search_largest_subset` finds, in alphabetical order.
fn search_largest_subset(
    trie: &FlatTrie<&'static str>,
    superset: &str,
    max_length: usize,
) -> Vec<&'static str> {
    let mut found = vec![];
    trie.search_largest_subset(LetterSet::from_str(superset), max_length, &mut found);
    found.sort();
    found
}

#[test]
fn search_largest_subset_finds_only_the_longest_keys_that_fit() {
    // "abce" needs an e, so the longest keys that fit hold three letters.
    let trie = build_word_trie(&["ab", "abc", "abd", "abce"]);
    assert_eq!(search_largest_subset(&trie, "abcd", 4), ["abc", "abd"]);
}

#[test]
fn search_largest_subset_skips_keys_longer_than_the_maximum() {
    // "abc" fits, but holds more than two letters.
    let trie = build_word_trie(&["ab", "abc"]);
    assert_eq!(search_largest_subset(&trie, "abc", 2), ["ab"]);
}
