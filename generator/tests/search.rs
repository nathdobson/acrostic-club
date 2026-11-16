// Times repeated searches of the trie table for one set of letters. Reads the trie table from
// `build/`.

use std::time::Instant;

use acrostic_core::letter::{Letter, LetterSet};
use acrostic_generator::trie_table::FLAT_TRIE_TABLE;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
#[ignore = "needs the generator's outputs in build/"]
async fn ten_thousand_trie_table_searches_finish() -> anyhow::Result<()> {
    let first_letters = (
        Letter::new(b'e').expect("e should be an ASCII letter"),
        Letter::new(b'i').expect("i should be an ASCII letter"),
    );
    let letters = LetterSet::from_str("AEEEEEEEEEEEGIIKKKKLLLNNNOPPTTTWWWW");
    FLAT_TRIE_TABLE.get().await.clone_error()?;

    let start = Instant::now();
    for _ in 0..10000 {
        let mut found = vec![];
        FLAT_TRIE_TABLE
            .get()
            .await
            .clone_error()?
            .binary
            .get(&first_letters)
            .expect("the trie table should hold a trie for each pair of first letters")
            .search_smallest_subset(letters, 16, &mut found);
    }

    let elapsed = start.elapsed();
    println!("{elapsed:?}");
    Ok(())
}
