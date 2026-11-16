// Tests for `CLUE_DB`: prints the stored clues for one answer. Reads the clue database from
// `build/xd/clues.tsv`.

use acrostic_generator::cluedb::CLUE_DB;
use acrostic_generator::string::LetterString;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
async fn test_cluedb() -> anyhow::Result<()> {
    let clue_db = CLUE_DB.get().await.clone_error_static()?;
    println!("{:?}", clue_db.lookup(&LetterString::from_str("howell")));
    Ok(())
}
