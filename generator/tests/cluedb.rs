// Tests for `CLUE_DB`: prints the stored clues for one answer. Reads the clue database from
// `build/xd/clues.tsv`.

use acrostic_generator::cluedb::CLUE_DB;
use acrostic_generator::string::LetterString;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
async fn the_clue_database_loads() -> anyhow::Result<()> {
    let clue_database = CLUE_DB.get().await.clone_error_static()?;
    let clues = clue_database.lookup(&LetterString::from_str("howell"));
    println!("{clues:?}");
    Ok(())
}
