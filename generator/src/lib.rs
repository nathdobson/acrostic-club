#![expect(dead_code)]
#![feature(error_generic_member_access)]
extern crate core;

use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{env, io};

use acrostic_core::letter::LetterSet;

pub mod add_letters;
pub mod banned;
pub mod cluebank;
pub mod cluedb;
pub mod clues;
pub mod command_line;
pub mod conflict_set;
pub mod dict;
pub mod lemma;
pub mod llm;
pub mod maintenance;
pub mod ontology;
pub mod puzzle;
pub mod quote;
pub mod reformat;
pub mod search;
pub mod site;
pub mod string;
pub mod subseq;
pub mod trie;
pub mod trie_table;
pub mod turtle;
pub mod util;
pub mod words_and_marks;

pub static PACKAGE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::var("CARGO_MANIFEST_DIR")
        .map(|x| PathBuf::from(x).join(".."))
        .unwrap_or(PathBuf::from(env::current_dir().unwrap()));
    println!("PACKAGE_PATH = {path:?}");
    path
});

// Returns a function for `map_err` that replaces an error with a new I/O error whose message adds
// `action` and `path`, since the original message, such as "No such file or directory", names
// neither. It accepts any error that converts into an I/O error, such as a JSON parse error. The
// new error copies the converted error's `io::ErrorKind`, such as `NotFound` or `InvalidData`, so
// callers can still check `kind()`; the original error survives only as text in the message.
pub fn error_with_path<E: Into<io::Error>>(
    action: &str,
    path: &Path,
) -> impl FnOnce(E) -> io::Error {
    move |error| {
        let error = error.into();
        io::Error::new(error.kind(), format!("Cannot {action} {path:?}: {error}"))
    }
}

pub async fn read_path(path: &Path) -> io::Result<Vec<u8>> {
    tokio::fs::read(path)
        .await
        .map_err(error_with_path("read", path))
}

pub async fn read_path_to_string(path: &Path) -> io::Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(error_with_path("read", path))
}

pub async fn write_path(path: &Path, x: &[u8]) -> io::Result<()> {
    tokio::fs::write(path, x)
        .await
        .map_err(error_with_path("write", path))
}

const QUOTE: &str = concat!(
    "And you've got to put your bodies upon the gears and ",
    "upon the wheels, upon the levers, ",
    "upon all the apparatus, and you've got to make it stop!",
    "...",
    "unless you're free the machine will be prevented from working at all",
);

const AUTHOR_TITLE: &str = concat!("Savio, Sprout Hall Address");

fn quote_letters() -> LetterSet {
    LetterSet::from_str(QUOTE)
}

fn author_title_letters() -> LetterSet {
    LetterSet::from_str(AUTHOR_TITLE)
}
