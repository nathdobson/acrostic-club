// Tests for `Watchable`: a watcher receives the changes made since its last receive, and an error
// once the `Watchable` is dropped.

use std::sync::mpsc::RecvError;
use std::time::Duration;

use acrostic_server::watch::Watchable;
use tokio::time::sleep;

#[tokio::test]
async fn test() {
    let mut sender = Watchable::<usize>::new();
    let r1 = sender.watch();
    let r2 = sender.watch();
    sender.modify(|x| *x += 1);
    let x = tokio::spawn(async move {
        assert_eq!(1, r1.recv().await.unwrap());
        assert_eq!(2, r1.recv().await.unwrap());
        assert_eq!(2, r1.recv().await.unwrap());
        assert_eq!(RecvError, r1.recv().await.unwrap_err());
    });
    let y = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        sender.modify(|x| *x += 2);
        sleep(Duration::from_millis(100)).await;
        sender.modify(|x| *x += 2);
    });
    x.await.unwrap();
    y.await.unwrap();
    assert_eq!(5, r2.recv().await.unwrap());
    assert_eq!(RecvError, r2.recv().await.unwrap_err());
}
