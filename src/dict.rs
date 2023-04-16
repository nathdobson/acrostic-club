use std::future::Future;
use std::io;
use std::sync::LazyLock;
use arrayvec::{ArrayString, ArrayVec};
use async_once::AsyncOnce;
use futures::future::Shared;
use futures::FutureExt;
use tokio::sync::Mutex;

use crate::alloc::{AnyRepr, MmapAllocator, restore_vec, save_vec};
use crate::{Letter, LetterSet, PACKAGE_PATH, read_path_to_string};

#[derive(Debug)]
#[repr(C)]
pub struct FlatWord {
    pub word: ArrayString<256>,
    pub letter_vec: ArrayVec<Letter, 128>,
    pub letters: LetterSet,
    pub frequency: u64,
}

unsafe impl AnyRepr for FlatWord {}

impl FlatWord {
    pub async fn get() -> io::Result<&'static [Self]> {
        let mut lock = FLAT_WORDS.lock().await;
        if lock.is_none() {
            *lock = Some(Box::leak(restore_vec(&PACKAGE_PATH.join("build/dict.dat")).await?) as &'static [FlatWord])
        }
        Ok(lock.as_ref().unwrap())
    }
}

static FLAT_WORDS: LazyLock<Mutex<Option<&'static [FlatWord]>>> = LazyLock::new(|| Mutex::new(None));


#[test]
fn test_flat_word() {
    println!("{:?}", FlatWord::get());
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
        let mut letter_vec = ArrayVec::new();
        let mut letters = LetterSet::new();
        for c in any_ascii::any_ascii(&word).chars() {
            if let Ok(letter) = Letter::new(c.try_into().unwrap()) {
                letter_vec.push(letter);
                letters[letter] += 1;
            }
        }
        words.push(FlatWord {
            word: (*word.to_string()).try_into().unwrap(),
            letter_vec,
            letters,
            frequency: freq as u64,
        })
    }

    save_vec(&PACKAGE_PATH.join("build/dict.dat"), &words).await?;
    Ok(())
}