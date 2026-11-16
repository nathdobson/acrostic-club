// Times repeated searches of the trie table for one set of letters. Reads the trie table from
// `build/`.

use std::time::Instant;

use acrostic_core::letter::{Letter, LetterSet};
use acrostic_generator::trie_table::FLAT_TRIE_TABLE;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
async fn test_search() -> anyhow::Result<()> {
    let firsts = (Letter::new(b'e').unwrap(), Letter::new(b'i').unwrap());
    let word = LetterSet::from_str("AEEEEEEEEEEEGIIKKKKLLLNNNOPPTTTWWWW");
    FLAT_TRIE_TABLE.get().await.clone_error()?;
    let start = Instant::now();
    for i in 0..10000 {
        let mut found = vec![];
        FLAT_TRIE_TABLE
            .get()
            .await
            .clone_error()?
            .binary
            .get(&firsts)
            .unwrap()
            .search_smallest_subset(word, 16, &mut found);
    }
    println!("{:?}", start.elapsed());
    Ok(())
}
