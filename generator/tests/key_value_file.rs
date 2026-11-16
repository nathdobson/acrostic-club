// Tests for `KeyValueFile`: values stored in the file come back after the file is reopened.

use std::mem;

use acrostic_generator::llm::key_value_file::KeyValueFile;
use acrostic_generator::util::interrupt::channel;
use tempfile::tempdir;

#[tokio::test]
async fn test_key_value_file() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("my-temporary-note.txt");
    let (tx, rx) = channel();
    let mut kvf = KeyValueFile::<usize, usize>::new(&path, tx).await?;
    assert_eq!(
        2,
        *kvf.get_or_init(1, async { anyhow::Result::Ok(2usize) })
            .await
            .unwrap()
    );
    assert_eq!(
        4,
        *kvf.get_or_init(3, async { anyhow::Result::Ok(4usize) })
            .await
            .unwrap()
    );
    mem::drop(kvf);
    rx.cleanup().await?;

    let (tx, rx) = channel();
    kvf = KeyValueFile::new(&path, tx).await?;
    assert_eq!(
        2,
        *kvf.get_or_init(1, async { anyhow::Result::Ok(todo!()) })
            .await
            .unwrap()
    );
    assert_eq!(
        4,
        *kvf.get_or_init(3, async { anyhow::Result::Ok(todo!()) })
            .await
            .unwrap()
    );
    mem::drop(kvf);
    rx.cleanup().await?;
    Ok(())
}
