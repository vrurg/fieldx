#![cfg(feature = "async")]
use fieldx::r#async::FXRwLockAsync;

#[tokio::test]
async fn rwlock() {
    let rwlock = FXRwLockAsync::new("initial".to_string());

    assert_eq!(*rwlock.read().await, "initial");

    {
        *rwlock.write().await = "new".to_string();
    }
    assert_eq!(*rwlock.read().await, "new");
}
