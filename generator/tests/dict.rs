// Tests for the word list that the `dict` step builds: prints the first few words of
// `build/dict.dat`.

use acrostic_generator::dict::FLAT_WORDS;

#[tokio::test]
async fn test_flat_word() {
    let words = FLAT_WORDS.get_static().await.unwrap();
    println!("{:?}", words.len());
    for word in words.iter().take(10) {
        println!("{:?}", word);
    }
}
