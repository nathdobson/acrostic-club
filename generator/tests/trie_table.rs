// Tests for the `trie_table` module. The first test below is commented out because it doesn't
// compile: it calls `FlatTrieTable::new` synchronously, but that constructor is private and async.

use acrostic_core::letter::LetterSet;
use acrostic_generator::trie::FlatTrie;
use acrostic_generator::trie_table;

// #[test]
// fn test_flat_trie_table() {
//     let table = FlatTrieTable::new();
//     for x in table.unary.iter() {
//         println!("{:?}", x.0);
//         format!("{:?}", x.1);
//     }
// }

// Builds a trie that holds each word's letters as both its key and its value, as the single-word
// tries do.
fn build_letter_trie(words: &[&str]) -> Box<FlatTrie<LetterSet>> {
    words
        .iter()
        .map(|word| (LetterSet::from_str(word), LetterSet::from_str(word)))
        .collect()
}

// Returns the pairs `trie_table::search_word_pairs` finds, sorted.
fn search_word_pairs(
    first_words: &[&str],
    second_words: &[&str],
    available_letters: &str,
    min_length: usize,
) -> Vec<(LetterSet, LetterSet)> {
    let mut found = vec![];
    trie_table::search_word_pairs(
        &build_letter_trie(first_words),
        &build_letter_trie(second_words),
        LetterSet::from_str(available_letters),
        min_length,
        &mut found,
    );
    found.sort();
    found
}

// Returns the letters of a pair of words, in the form the search returns them.
fn letter_pair(first_word: &str, second_word: &str) -> (LetterSet, LetterSet) {
    (
        LetterSet::from_str(first_word),
        LetterSet::from_str(second_word),
    )
}

#[test]
fn search_word_pairs_finds_pairs_whose_letters_together_are_a_subset() {
    // "ab" and "cd" use each letter of "abcd" once; any other pair needs a second c or an e.
    let found = search_word_pairs(&["ab", "ac"], &["cd", "ce"], "abcd", 0);
    assert_eq!(found, [letter_pair("ab", "cd")]);
}

#[test]
fn search_word_pairs_skips_pairs_shorter_than_the_minimum() {
    // "ab" and "c" hold only three letters.
    let found = search_word_pairs(&["ab"], &["c", "cd"], "abcd", 4);
    assert_eq!(found, [letter_pair("ab", "cd")]);
}

#[test]
fn search_word_pairs_accepts_any_second_word_once_the_first_reaches_the_minimum() {
    // "abcd" alone holds the three letters required, so even "e" can follow it.
    let found = search_word_pairs(&["abcd"], &["e", "ef"], "abcdef", 3);
    assert_eq!(found, [letter_pair("abcd", "e"), letter_pair("abcd", "ef")]);
}
