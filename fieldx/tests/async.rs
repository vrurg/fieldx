#![cfg(feature = "async")]
use core::time;
use fieldx::fxstruct;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Barrier;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::sleep;

#[fxstruct(mode(async))]
#[derive(Debug)]
pub struct Foo {
    #[fieldx(lazy, writer, clearer, predicate)]
    foo: String,
    #[fieldx(lazy, writer, predicate, clearer, set)]
    bar: i32,

    // Let's try a charged but not lazy field
    #[fieldx(r#async, reader, writer, get(off), clearer, predicate, set, default("bazzification".to_string()))]
    baz: String,

    #[fieldx(lazy, clearer, default("fufubarik!".to_string()))]
    fubar: String,

    bar_builds: Mutex<i32>,
    next_bar:   Mutex<Option<i32>>,

    #[fieldx(writer, get(copy), get_mut, default(54321))]
    can_copy: i32,
}

impl Foo {
    async fn build_foo(&self) -> String {
        format!("Foo with bar={:?}", *self.bar().await).to_string()
    }

    async fn build_bar(&self) -> i32 {
        *self.bar_builds.lock().await += 1;
        self.next_bar.lock().await.unwrap_or(42)
    }

    async fn build_fubar(&self) -> String {
        "аби було".to_string()
    }

    pub async fn set_next_bar(&self, next_val: i32) {
        *self.next_bar.lock().await = Some(next_val);
    }

    pub async fn bar_builds(&self) -> i32 {
        *self.bar_builds.lock().await
    }
}

#[tokio::test]
async fn non_threaded() {
    let foo_async = Foo::new();

    assert!(!foo_async.has_foo(), "foo is not initialized yet");
    assert_eq!(*foo_async.foo().await, "Foo with bar=42".to_string(), "built foo");
    assert!(foo_async.has_foo(), "foo has been built");
    assert!(foo_async.has_bar(), "bar has been built");
    assert_eq!(foo_async.clear_bar().await, Some(42), "cleared bar");
    assert!(!foo_async.has_bar(), "bar has been cleared");
    assert_eq!(
        *foo_async.foo().await,
        "Foo with bar=42".to_string(),
        "foo is unchanged after clearng bar"
    );
    assert!(
        !foo_async.has_bar(),
        "reading uncleared foo does not trigger bar building"
    );
    assert_eq!(foo_async.set_bar(12).await, None, "set bar");
    assert!(foo_async.has_bar(), "bar now has a value");
    assert_eq!(
        foo_async.clear_foo().await,
        Some(String::from("Foo with bar=42")),
        "cleared foo"
    );
    assert!(!foo_async.has_foo(), "foo has been cleared");
    assert_eq!(
        *foo_async.foo().await,
        String::from("Foo with bar=12"),
        "manually set bar is used to rebuild foo"
    );

    {
        let mut wrbar = foo_async.write_bar().await;
        wrbar.store(666);
        foo_async.clear_foo().await;
    }

    assert_eq!(
        *foo_async.foo().await,
        String::from("Foo with bar=666"),
        "manually set bar using write lock"
    );

    assert_eq!(
        *foo_async.fubar().await,
        String::from("fufubarik!"),
        "fubar initial values is the default"
    );
    assert_eq!(
        foo_async.clear_fubar().await,
        Some(String::from("fufubarik!")),
        "cleared fubar"
    );
    assert_eq!(*foo_async.fubar().await, String::from("аби було"), "built fubar");

    assert_eq!(foo_async.can_copy().await, 54321, "can_copy is 54321");
    *foo_async.write_can_copy().await = 12345;
    assert_eq!(foo_async.can_copy().await, 12345, "can_copy is 12345");
}

#[tokio::test]
async fn non_lazy() {
    let foo_async = Foo::new();

    assert_eq!(
        *foo_async.read_baz().await,
        Some(String::from("bazzification")),
        "initially set to a default"
    );
    assert_eq!(
        foo_async.clear_baz().await,
        Some(String::from("bazzification")),
        "cleared"
    );
    assert!(!foo_async.has_baz().await, "empty after clear");
    {
        let mut wrg = foo_async.write_baz().await;
        *wrg = Some("bazzish".to_string());
    }
    assert_eq!(
        *foo_async.read_baz().await,
        Some(String::from("bazzish")),
        "set to a new value"
    );
    assert_eq!(
        foo_async.set_baz("bazzuka".to_string()).await,
        Some("bazzish".to_string()),
        "setter returns old value"
    );
    assert_eq!(
        *foo_async.read_baz().await,
        Some(String::from("bazzuka")),
        "set with a setter"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn threaded() {
    let thread_count = num_cpus::get() * 2;
    let sync = Arc::new(Foo::new());
    let wg = Arc::new(Barrier::new(thread_count + 1));
    let stop = Arc::new(AtomicBool::new(false));
    let next_bar = Arc::new(Mutex::new(100));
    let cleared = Arc::new(AtomicI32::new(0));
    let expected_foo = Arc::new(Mutex::new(String::from("Foo with bar=42")));
    let mut thandles = JoinSet::new();

    for thread_id in 0..thread_count {
        let scopy = Arc::clone(&sync);
        let twg = wg.clone();
        let tstop = stop.clone();
        let texpect = expected_foo.clone();
        let tcleared = cleared.clone();
        let tnext_bar = next_bar.clone();

        thandles.spawn(async move {
            eprintln!("[{thread_id:>4}] started");
            twg.wait().await;
            let mut i = 0;
            'main: loop {
                i += 1;
                eprintln!("[{:>4}] {:?}", thread_id, scopy.foo().await.clone());
                assert_eq!(*scopy.foo().await, (*texpect.lock().await.clone()), "foo value");
                if (i % 13) == 0 {
                    eprintln!("Time to clear for {} since {} % 13 == {}", thread_id, i, i % 13);
                    // Prevent other threads from accessing foo untils we're done updating bar
                    let lock_foo = scopy.write_foo().await;
                    let mut wnext = tnext_bar.lock().await;
                    *wnext += 1;
                    scopy.set_next_bar(*wnext).await;
                    scopy.clear_bar().await;
                    assert!(!scopy.has_bar(), "bar is cleared and stays so until foo is unlocked");
                    tcleared.fetch_add(1, Ordering::SeqCst);
                    *texpect.lock().await = format!("Foo with bar={}", *wnext).to_string();
                    lock_foo.clear();
                    eprintln!(
                        "[{:>4}] Now should expect for '{}' // {}",
                        thread_id,
                        *texpect.lock().await,
                        scopy.has_foo()
                    );

                    // Ensure that we always perform at least one clear after a build. This guarantee holds only if
                    // we check the stop flag here, after incrementing the clear counter.
                    if tstop.load(Ordering::Relaxed) {
                        break 'main;
                    }
                }
            }
            eprintln!("[{thread_id:>4}] done");
            thread_id
        });
    }

    wg.wait().await;

    while cleared.load(Ordering::SeqCst) < thread_count as i32 * 50 {
        sleep(time::Duration::from_millis(10)).await;
    }

    sleep(time::Duration::from_millis(500)).await;
    stop.store(true, Ordering::Relaxed);
    thandles.join_all().await;
    let clear_count = cleared.load(Ordering::SeqCst);
    let build_count = sync.bar_builds().await;
    eprintln!("cleared {clear_count} times, built {build_count} times");
    assert!(
        clear_count >= build_count,
        "there were less clears than builds ({clear_count} vs. {build_count})"
    );
}
