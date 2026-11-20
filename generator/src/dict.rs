use std::io;

use crate::banned::BANNED_WORDS;
use crate::{PACKAGE_PATH, read_path_to_string};
use acrostic_core::letter::{Letter, LetterSet};

use crate::util::persist::PersistentFile;
use rkyv::Archive;
use safe_once::sync::LazyLock;

#[derive(Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive(check_bytes, archived = "FlatWord")]
#[archive_attr(derive(Debug))]
pub struct FlatWordBuilder {
    pub word: String,
    pub letter_vec: Vec<Letter>,
    pub letters: LetterSet,
    pub frequency: u64,
}

pub static FLAT_WORDS: LazyLock<PersistentFile<Vec<FlatWordBuilder>>> =
    LazyLock::new(|| PersistentFile::new(&PACKAGE_PATH.join("build/dict.dat")));

pub async fn build_dict() -> io::Result<()> {
    let path =
        PACKAGE_PATH.join("submodules/wikipedia-word-frequency/results/enwiki-2022-08-29.txt");
    let contents = read_path_to_string(&path).await?;

    let mut words = vec![];
    for line in contents.split("\n") {
        if line.is_empty() {
            continue;
        }
        // The line itself shows what's wrong with it, so the error leaves out the parser's reason.
        let malformed_line_error = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Cannot parse {path:?}: expected a word, a space, and a count in {line:?}"),
            )
        };
        let (word, freq) = line.split_once(" ").ok_or_else(malformed_line_error)?;
        let freq: usize = freq.parse().map_err(|_| malformed_line_error())?;
        let mut letter_vec = Vec::new();
        let mut letters = LetterSet::new();
        for c in any_ascii::any_ascii(&word).chars() {
            if let Ok(letter) =
                Letter::new(c.try_into().expect("`any_ascii` should return only ASCII"))
            {
                letter_vec.push(letter);
                letters[letter] += 1;
            }
        }
        if !BANNED_WORDS.contains(word) {
            words.push(FlatWordBuilder {
                word: word.to_string(),
                letter_vec,
                letters,
                frequency: freq as u64,
            });
        }
    }

    FLAT_WORDS.set(&words).await?;
    Ok(())
}
