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

// Returns the words `FlatTrie::search_subsets_with_min_length` finds, in alphabetical order.
fn search_subsets_with_min_length(
    trie: &FlatTrie<&'static str>,
    superset: &str,
    min_length: usize,
) -> Vec<&'static str> {
    let mut found = vec![];
    trie.search_subsets_with_min_length(LetterSet::from_str(superset), min_length, &mut found);
    found.sort();
    found
}

#[test]
fn search_subsets_with_min_length_skips_keys_shorter_than_the_minimum() {
    let trie = build_word_trie(&["ab", "abc", "abce"]);
    assert_eq!(
        search_subsets_with_min_length(&trie, "abce", 3),
        ["abc", "abce"]
    );
}

#[test]
fn search_subsets_with_min_length_skips_keys_the_superset_cannot_hold() {
    // "abdd" needs a second d.
    let trie = build_word_trie(&["abc", "abd", "abdd"]);
    assert_eq!(
        search_subsets_with_min_length(&trie, "abcd", 0),
        ["abc", "abd"]
    );
}

#[test]
fn search_subsets_with_min_length_counts_every_unused_copy_of_a_letter() {
    // "bc" leaves both a's unused, but a minimum length of three allows only one unused letter.
    let trie = build_word_trie(&["bc", "abc", "aabc"]);
    assert_eq!(
        search_subsets_with_min_length(&trie, "aabc", 3),
        ["aabc", "abc"]
    );
}

#[test]
fn search_subsets_with_min_length_finds_nothing_when_the_minimum_exceeds_the_superset() {
    let trie = build_word_trie(&["ab", "abc"]);
    assert!(search_subsets_with_min_length(&trie, "ab", 3).is_empty());
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
