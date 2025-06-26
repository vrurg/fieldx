#![cfg(all(feature = "async", feature = "clonable-lock"))]
use fieldx::r#async::FXRwLock;

#[tokio::test]
async fn rwlock() {
    let rwlock = FXRwLock::new("initial".to_string());

    assert_eq!(*rwlock.read().await, "initial");

    {
        *rwlock.write().await = "new".to_string();
    }
    assert_eq!(*rwlock.read().await, "new");
}
