#![cfg(feature = "sync")]
use core::time;
use crossbeam::sync;
use crossbeam::thread;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
mod syncish;

#[test]
fn non_threaded() {
    let mut sync = syncish::Foo::new();

    assert!(!sync.has_foo(), "foo is not initialized yet");
    assert_eq!(*sync.foo(), "Foo with bar=42".to_string(), "built foo");
    assert!(sync.has_foo(), "foo has been built");
    assert!(sync.has_bar(), "bar has been built");
    assert_eq!(sync.clear_bar(), Some(42), "cleared bar");
    assert!(!sync.has_bar(), "bar has been cleared");
    assert_eq!(
        *sync.foo(),
        "Foo with bar=42".to_string(),
        "foo is unchanged after clearng bar"
    );
    assert!(!sync.has_bar(), "reading uncleared foo does not trigger bar building");
    assert_eq!(sync.set_bar(12), None, "set bar");
    assert!(sync.has_bar(), "bar now has a value");
    assert_eq!(sync.clear_foo(), Some(String::from("Foo with bar=42")), "cleared foo");
    assert!(!sync.has_foo(), "foo has been cleared");
    assert_eq!(
        *sync.foo(),
        String::from("Foo with bar=12"),
        "manually set bar is used to rebuild foo"
    );

    {
        let mut wrbar = sync.write_bar();
        wrbar.store(666);
        sync.clear_foo();
    }

    assert_eq!(
        *sync.foo(),
        String::from("Foo with bar=666"),
        "manually set bar using write lock"
    );

    assert_eq!(
        *sync.fubar(),
        String::from("fufubarik!"),
        "fubar initial values is the default"
    );
    assert_eq!(sync.clear_fubar(), Some(String::from("fufubarik!")), "cleared fubar");
    assert_eq!(*sync.fubar(), String::from("аби було"), "built fubar");
}

#[test]
fn non_lazy() {
    let sync = syncish::Foo::new();

    assert_eq!(
        *sync.read_baz(),
        Some(String::from("bazzification")),
        "initially set to a default"
    );
    assert_eq!(sync.clear_baz(), Some(String::from("bazzification")), "cleared");
    assert!(!sync.has_baz(), "empty after clear");
    {
        let mut wrg = sync.write_baz();
        *wrg = Some("bazzish".to_string());
    }
    assert_eq!(*sync.read_baz(), Some(String::from("bazzish")), "set to a new value");
    assert_eq!(
        sync.set_baz("bazzuka".to_string()),
        Some("bazzish".to_string()),
        "setter returns old value"
    );
    assert_eq!(*sync.read_baz(), Some(String::from("bazzuka")), "set with a setter");
}

#[test]
fn threaded() {
    thread::scope(|s| {
        let thread_count = num_cpus::get() * 2;
        let foo = Arc::new(syncish::Foo::new());
        let wg = sync::WaitGroup::new();
        let stop = Arc::new(AtomicBool::new(false));
        let next_bar = Arc::new(Mutex::new(100));
        let cleared = Arc::new(AtomicI32::new(0));
        let expected_foo = Arc::new(Mutex::new(String::from("Foo with bar=42")));
        let mut thandles: Vec<thread::ScopedJoinHandle<()>> = vec![];

        for thread_id in 0..thread_count {
            let thread_foo = Arc::clone(&foo);
            let twg = wg.clone();
            let tstop = stop.clone();
            let texpect = expected_foo.clone();
            let tcleared = cleared.clone();
            let tnext_bar = next_bar.clone();

            thandles.push(s.spawn(move |_| {
                twg.wait();
                let mut i = 0;
                'main: loop {
                    i += 1;
                    eprintln!("[{:>4}] {:?}", thread_id, thread_foo.foo().clone());
                    assert_eq!(*thread_foo.foo(), (*texpect.lock().clone()), "foo value");
                    if (i % 13) == 0 {
                        // Prevent other threads from accessing foo until we're done updating bar
                        let lock_foo = thread_foo.write_foo();
                        eprintln!("[{:>4}] Time to clear since {} % 13 == {}", thread_id, i, i % 13);
                        let mut wnext = tnext_bar.lock();
                        *wnext += 1;
                        thread_foo.set_next_bar(*wnext);
                        thread_foo.clear_bar();
                        tcleared.fetch_add(1, Ordering::SeqCst);
                        *texpect.lock() = format!("Foo with bar={}", *wnext).to_string();
                        lock_foo.clear();
                        eprintln!(
                            "[{:>4}] Now should expect for '{}' // {}",
                            thread_id,
                            *texpect.lock(),
                            thread_foo.has_foo()
                        );

                        // Ensure that we always perform at least one clear after a build.  This guarantee holds only if
                        // we check the stop flag here, after incrementing the clear counter.
                        if tstop.load(Ordering::SeqCst) {
                            break 'main;
                        }
                    }
                }
                eprintln!("[{thread_id:>4}] done");
            }));
        }

        wg.wait();

        // Wait until each thread has performed at least 50 clears on average.
        // Avoid using exact numbers to introduce more randomness.
        while cleared.load(Ordering::Relaxed) < thread_count as i32 * 50 {
            std::thread::sleep(time::Duration::from_millis(10));
        }

        stop.store(true, Ordering::SeqCst);
        for thandle in thandles {
            thandle.join().expect("thread join failed");
        }
        // There is a chance for two or more consecutive clears to take place before a build is invoked.
        let clear_count = cleared.load(Ordering::SeqCst);
        let build_count = foo.bar_builds();
        eprintln!("clears: {clear_count}, builds: {build_count}");
        assert!(
            clear_count >= build_count,
            "there were less clears than builds ({clear_count} < {build_count}) - this must not happen."
        );
    })
    .unwrap();
}
