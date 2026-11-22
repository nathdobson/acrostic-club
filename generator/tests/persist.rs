// Tests for `mmap_bytes`, which maps a saved output of the generator, such as the word list, into
// memory.

use std::io;

use acrostic_generator::util::persist::mmap_bytes;
use tempfile::tempdir;

#[tokio::test]
async fn mapping_a_missing_file_returns_a_not_found_error() {
    let directory = tempdir().expect("a temporary directory should be creatable");
    let path = directory.path().join("dict.dat");

    let error = mmap_bytes(&path)
        .await
        .expect_err("mapping a missing file should fail");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.to_string().contains("dict.dat"), "{error}");
}
