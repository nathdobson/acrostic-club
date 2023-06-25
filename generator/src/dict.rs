use std::future::Future;
use std::io;
use std::sync::LazyLock;

use arrayvec::{ArrayString, ArrayVec};
use async_once::AsyncOnce;
use futures::future::Shared;
use futures::FutureExt;
use tokio::sync::Mutex;
use acrostic_core::letter::{Letter, LetterSet};
use crate::{PACKAGE_PATH, read_path_to_string};
use crate::banned::BANNED_WORDS;

use crate::util::alloc::{AnyRepr, restore_rkyv, save_rkyv, save_vec};
use crate::util::lazy_async::{CloneError, LazyMmap};
use rkyv::{Archive, Archived};
use safe_once_async::async_lazy::AsyncLazy;
use safe_once_async::async_static::AsyncStatic;
use safe_once_async::sync::AsyncStaticLock;

#[derive(Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive(check_bytes, archived = "FlatWord")]
pub struct FlatWordBuilder {
    pub word: String,
    pub letter_vec: Vec<Letter>,
    pub letters: LetterSet,
    pub frequency: u64,
}

unsafe impl AnyRepr for FlatWord {}

// pub static FLAT_WORDS: LazyMmap<FlatWord> =
//     LazyMmap::<FlatWord>::new(|| PACKAGE_PATH.join("build/dict.dat"));

pub static FLAT_WORDS: AsyncStaticLock<io::Result<&'static Archived<Vec<FlatWordBuilder>>>> = AsyncStatic::new(async {
    restore_rkyv::<Vec<FlatWordBuilder>>(&PACKAGE_PATH.join("build/dict.dat")).await
});

#[tokio::test]
async fn test_flat_word() {
    println!("{:?}", FLAT_WORDS.get().await.clone_error().unwrap().len());
}

pub async fn build_dict() -> io::Result<()> {
    let contents = read_path_to_string(
        &PACKAGE_PATH.join("submodules/wikipedia-word-frequency/results/enwiki-2022-08-29.txt"),
    ).await?;

    let mut words = vec![];
    for line in contents.split("\n") {
        if line.is_empty() {
            continue;
        }
        let (word, freq) = line.split_once(" ").unwrap();
        let freq: usize = freq.parse().unwrap();
        let mut letter_vec = Vec::new();
        let mut letters = LetterSet::new();
        for c in any_ascii::any_ascii(&word).chars() {
            if let Ok(letter) = Letter::new(c.try_into().unwrap()) {
                letter_vec.push(letter);
                letters[letter] += 1;
            }
        }
        if !BANNED_WORDS.contains(word) {
            words.push(FlatWordBuilder {
                word: (*word.to_string()).try_into().unwrap(),
                letter_vec,
                letters,
                frequency: freq as u64,
            });
        }
    }

    save_rkyv(&PACKAGE_PATH.join("build/dict.dat"), &words).await?;
    Ok(())
}
