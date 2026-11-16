// Tests for `Watchable`: a watcher receives the changes made since its last receive, and an error
// once the `Watchable` is dropped.

use std::sync::mpsc::RecvError;
use std::time::Duration;

use acrostic_server::watch::Watchable;
use tokio::time::sleep;

#[tokio::test]
async fn watchers_receive_changes_until_the_watchable_drops() {
    let mut watchable = Watchable::<usize>::new();
    let first_watcher = watchable.watch();
    let second_watcher = watchable.watch();
    watchable.modify(|value| *value += 1);
    let first_watcher_task = tokio::spawn(async move {
        let watchable_exists_message = "the watchable should still exist";
        assert_eq!(
            1,
            first_watcher.recv().await.expect(watchable_exists_message)
        );
        assert_eq!(
            2,
            first_watcher.recv().await.expect(watchable_exists_message)
        );
        assert_eq!(
            2,
            first_watcher.recv().await.expect(watchable_exists_message)
        );
        assert_eq!(
            RecvError,
            first_watcher
                .recv()
                .await
                .expect_err("the watchable should have dropped")
        );
    });
    let modifying_task = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        watchable.modify(|value| *value += 2);
        sleep(Duration::from_millis(100)).await;
        watchable.modify(|value| *value += 2);
    });

    first_watcher_task
        .await
        .expect("the first watcher's task should not panic");
    modifying_task
        .await
        .expect("the modifying task should not panic");

    assert_eq!(
        5,
        second_watcher
            .recv()
            .await
            .expect("the second watcher should still deliver the changes it has not received")
    );
    assert_eq!(
        RecvError,
        second_watcher
            .recv()
            .await
            .expect_err("the watchable should have dropped")
    );
}
