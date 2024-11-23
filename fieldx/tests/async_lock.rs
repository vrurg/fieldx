use fieldx::r#async::FXRwLockAsync;
use tokio;

#[tokio::test]
async fn rwlock() {
    let rwlock = FXRwLockAsync::new("initial".to_string());

    assert_eq!(*rwlock.read().await, "initial");

    {
        *rwlock.write().await = "new".to_string();
    }
    assert_eq!(*rwlock.read().await, "new");
}
