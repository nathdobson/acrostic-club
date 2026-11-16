// Tests for `LEMMA`: loads the list of word forms from `build/lemma.en.txt`.

use acrostic_generator::lemma::LEMMA;
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
async fn test_lemma() -> anyhow::Result<()> {
    let lemma = LEMMA.get().await.clone_error()?;
    Ok(())
}
