// Tests for `Lemma`: parses and loads the list of word forms from `build/lemma.en.txt`.

use acrostic_generator::lemma::{LEMMA, Lemma};
use acrostic_generator::util::lazy_async::CloneError;

#[tokio::test]
async fn the_lemma_list_loads() -> anyhow::Result<()> {
    LEMMA.get().await.clone_error()?;
    Ok(())
}

#[test]
fn parsing_trims_every_form_of_a_word() {
    let lemma = Lemma::parse("buy/25801 -> bought,buying,buys").expect("the line should parse");

    assert_eq!(lemma.alternates("buy"), ["bought", "buying", "buys"]);
    assert_eq!(lemma.canonicals("bought"), ["buy"]);
}

// Without a frequency tag, the space before the arrow follows the base word directly.
#[test]
fn parsing_trims_a_base_word_without_a_frequency() {
    let lemma = Lemma::parse("able -> abler,ablest").expect("the line should parse");

    assert_eq!(lemma.alternates("able"), ["abler", "ablest"]);
    assert_eq!(lemma.canonicals("ablest"), ["able"]);
}
