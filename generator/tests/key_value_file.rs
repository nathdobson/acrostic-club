// Tests for `KeyValueFile`: values stored in the file come back after the file is reopened.

use std::mem;

use acrostic_generator::llm::key_value_file::KeyValueFile;
use acrostic_generator::util::interrupt::channel;
use tempfile::tempdir;

#[tokio::test]
async fn stored_values_survive_reopening_the_file() -> anyhow::Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("my-temporary-note.txt");
    let (cleanup_sender, cleanup_receiver) = channel();
    let mut key_value_file = KeyValueFile::<usize, usize>::new(&path, cleanup_sender).await?;

    assert_eq!(
        2,
        *key_value_file
            .get_or_init(1, async { anyhow::Result::Ok(2usize) })
            .await
            .expect("a new key should take its initializer's value")
    );
    assert_eq!(
        4,
        *key_value_file
            .get_or_init(3, async { anyhow::Result::Ok(4usize) })
            .await
            .expect("a new key should take its initializer's value")
    );

    mem::drop(key_value_file);
    cleanup_receiver.cleanup().await?;

    let (cleanup_sender, cleanup_receiver) = channel();
    key_value_file = KeyValueFile::new(&path, cleanup_sender).await?;

    assert_eq!(
        2,
        *key_value_file
            .get_or_init(1, async { todo!() })
            .await
            .expect("a stored key should load its value from the file")
    );
    assert_eq!(
        4,
        *key_value_file
            .get_or_init(3, async { todo!() })
            .await
            .expect("a stored key should load its value from the file")
    );

    mem::drop(key_value_file);
    cleanup_receiver.cleanup().await?;
    Ok(())
}
