use std::collections::{HashMap, HashSet};
use std::io;
use crate::dict::FLAT_WORDS;
use crate::string::{GraphemeString, LetterString};

#[tokio::test]
async fn test() -> io::Result<()> {
    let dict = FLAT_WORDS.get().await?;
    let mut stems = HashSet::new();
    for (index, word) in dict.iter().enumerate() {
        let graphemes = GraphemeString::from_str(&word.word);
        for stem in graphemes.stems() {
            stems.insert(LetterString::from_graphemes(&stem.iter().cloned().collect()));
        }
    }
    let mut table = HashMap::<LetterString, usize>::new();
    for stem in stems {
        for start in 0..stem.len() {
            for end in start..=stem.len() {
                if end - start > 5 && end - start < 10 {
                    *table.entry(stem[start..end].iter().cloned().collect()).or_default() += 1;
                }
            }
        }
    }
    for i in 0..10 {
        let mut order: Vec<_> = table.iter().filter(|(k, v)| k.len() == i).collect();
        order.sort_by_key(|(k, v)| *v);
        println!("{:?}", i);
        for (k, v) in order.iter().rev().take(10) {
            println!("{:?} {:?}", k, v);
        }
    }
    Ok(())
}