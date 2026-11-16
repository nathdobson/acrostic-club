// Tests for the word list that the `dict` step builds: prints the first few words of
// `build/dict.dat`.

use acrostic_generator::dict::FLAT_WORDS;

#[tokio::test]
async fn the_word_list_loads() {
    let words = FLAT_WORDS
        .get_static()
        .await
        .expect("the word list should load from build/dict.dat");

    let word_count = words.len();
    println!("{word_count:?}");
    for word in words.iter().take(10) {
        println!("{word:?}");
    }
}
