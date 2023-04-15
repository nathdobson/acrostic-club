use arrayvec::{ArrayString, ArrayVec};

use crate::alloc::{restore_vec, AnyRepr, MmapAllocator};
use crate::{Letter, LetterSet, PACKAGE_PATH};

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
    // pub fn new(x: &DictWord) -> Self {
    //     FlatWord {
    //         word: (*x.word).try_into().unwrap(),
    //         letter_vec: (*x.letter_vec).try_into().unwrap(),
    //         letters: x.letters,
    //         frequency: 0,
    //     }
    // }
    pub fn get() -> Box<[Self], MmapAllocator> { restore_vec(&PACKAGE_PATH.join("index/dict.dat")) }
}

#[test]
fn test_flat_word() {
    println!("{:?}", FlatWord::get());
}
