// Tests for `FlatTrie`: prints a trie built from three words.

use acrostic_core::letter::LetterSet;
use acrostic_generator::trie::FlatTrie;

#[test]
fn test_flat_trie() {
    let mut entries = vec!["ab", "abc", "abd"];
    let b: Box<FlatTrie<&str>> = entries
        .into_iter()
        .map(|x| (LetterSet::from_str(x), x))
        .collect();
    println!("{:?}", b.as_slice());
    println!("{:?}", b);
}
