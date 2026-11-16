// Tests for `FlatTrie`: prints a trie built from three words.

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
