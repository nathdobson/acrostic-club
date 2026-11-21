// Tests for `FlatTrie` and `build_tries`.

use std::iter;

use acrostic_core::letter::{Letter, LetterSet};
use acrostic_generator::trie::{FlatTrie, build_tries};
use itertools::Itertools;

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

// Returns every value `trie` holds, in trie order.
fn list_values(trie: &FlatTrie<LetterSet>) -> Vec<LetterSet> {
    let every_letter = LetterSet::from_str(&"abcdefghijklmnopqrstuvwxyz".repeat(10));
    let mut found = vec![];
    trie.search_subsets_with_min_length(every_letter, 0, &mut found);
    found
}

// Returns the letter that `byte` spells, such as A for `b'a'`.
fn letter(byte: u8) -> Letter {
    Letter::new(byte).expect("a test byte should spell a letter")
}

// Pairs each of `words` with its first letter, the form `build_tries` takes.
fn make_words(words: &[&str]) -> Vec<(Letter, LetterSet)> {
    words
        .iter()
        .map(|word| (letter(word.as_bytes()[0]), LetterSet::from_str(word)))
        .collect()
}

#[test]
fn tries_keep_long_words_under_their_first_letter() {
    let tries = build_tries(make_words(&["letters", "lest", "average"]));

    // "lest" holds only four letters, too few for the tries.
    assert_eq!(
        list_values(&tries[letter(b'l')]),
        [LetterSet::from_str("letters")]
    );
    assert_eq!(
        list_values(&tries[letter(b'a')]),
        [LetterSet::from_str("average")]
    );
}

#[test]
fn tries_skip_a_word_whose_first_letter_and_letters_match_a_kept_word() {
    let tries = build_tries(make_words(&["abroad", "aboard"]));

    assert_eq!(
        list_values(&tries[letter(b'a')]),
        [LetterSet::from_str("abroad")]
    );
}

#[test]
fn tries_keep_words_that_share_their_letters_but_not_their_first_letter() {
    let tries = build_tries(make_words(&["listen", "silent"]));

    assert_eq!(
        list_values(&tries[letter(b'l')]),
        [LetterSet::from_str("listen")]
    );
    assert_eq!(
        list_values(&tries[letter(b's')]),
        [LetterSet::from_str("silent")]
    );
}

// Returns `count` words of six letters, each an a followed by five other letters in alphabetical
// order, such as "abcdef". No two of the words share their letters.
fn make_distinct_words(count: usize) -> Vec<String> {
    ('b'..='z')
        .combinations(5)
        .take(count)
        .map(|other_letters| iter::once('a').chain(other_letters).collect())
        .collect()
}

#[test]
fn a_skipped_word_leaves_its_slot_under_the_cap_to_a_later_word() {
    // The filler words hold one a each, and "abroad" and "average" hold two, so the filler words
    // share their letters with neither. With those two words, the 14,998 filler words fill the
    // 15,000 slots exactly.
    let filler_words = make_distinct_words(14_998);
    let mut words = vec!["abroad", "aboard"];
    words.extend(filler_words.iter().map(String::as_str));
    words.push("average");

    let tries = build_tries(make_words(&words));

    // "aboard" matches "abroad", so the slot it would take goes to "average".
    let a_values = list_values(&tries[letter(b'a')]);
    assert!(a_values.contains(&LetterSet::from_str("average")));
}
