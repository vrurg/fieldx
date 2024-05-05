use core::time;
use crossbeam::{sync, thread};
use num_cpus;
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc,
};
mod syncish;

#[test]
fn non_threaded() {
    let sync = syncish::Foo::new();
    // assert_eq!(*sync.read_foo(), Some("Foo with bar=Some(42)".to_string()));
    // assert_eq!(sync.bar(), Some(42));
    // assert_eq!(*sync.read_bar(), Some(42));
    // assert_eq!(sync.pi, 3.1415926);

    assert!(!sync.has_foo(), "foo is not initialized yet");
    assert_eq!(*sync.read_foo(), "Foo with bar=42".to_string(), "built foo");
    assert!(sync.has_foo(), "foo has been built");
    assert!(sync.has_bar(), "bar has been built");
    assert_eq!(sync.clear_bar(), Some(42), "cleared bar");
    assert!(!sync.has_bar(), "bar has been cleared");
    assert_eq!(
        *sync.read_foo(),
        "Foo with bar=42".to_string(),
        "foo is unchanged after clearng bar"
    );
    assert!(!sync.has_bar(), "reading uncleared foo does not trigger bar building");
    assert_eq!(sync.set_bar(12), None, "set bar");
    assert!(sync.has_bar(), "bar now has a value");
    assert_eq!(sync.clear_foo(), Some(String::from("Foo with bar=42")), "cleared foo");
    assert!(!sync.has_foo(), "foo has been cleared");
    assert_eq!(
        *sync.read_foo(),
        String::from("Foo with bar=12"),
        "manually set bar is used to rebuild foo"
    );

    {
        let mut wrbar = sync.write_bar();
        wrbar.store(666);
        sync.clear_foo();
    }

    assert_eq!(
        *sync.read_foo(),
        String::from("Foo with bar=666"),
        "manually set bar using write lock"
    );

    assert_eq!(*sync.read_fubar(), String::from("аби було"), "built fubar");
    assert_eq!(sync.clear_fubar(), Some(String::from("аби було")), "cleared fubar");
}

#[test]
fn non_lazy() {
    let sync = syncish::Foo::new();

    assert_eq!(
        *sync.read_baz(),
        String::from("bazzification"),
        "initially set to a default"
    );
    assert_eq!(sync.clear_baz(), Some(String::from("bazzification")), "cleared");
    assert!(!sync.has_baz(), "empty after clear");
    {
        let mut wrg = sync.write_baz();
        *wrg = Some("bazzish".to_string());
    }
    assert_eq!(*sync.read_baz(), String::from("bazzish"), "set to a new value");
    assert_eq!(
        sync.set_baz("bazzuka".to_string()),
        Some("bazzish".to_string()),
        "setter returns old value"
    );
    assert_eq!(*sync.read_baz(), String::from("bazzuka"), "set with a setter");
}

#[test]
fn threaded() {
    thread::scope(|s| {
        let thread_count = num_cpus::get() * 2;
        let sync = Arc::new(syncish::Foo::new());
        let wg = sync::WaitGroup::new();
        let stop = Arc::new(AtomicBool::new(false));
        let next_bar = Arc::new(Mutex::new(100));
        let cleared = Arc::new(AtomicI32::new(0));
        let expected_foo = Arc::new(Mutex::new(String::from("Foo with bar=42")));
        let mut thandles: Vec<thread::ScopedJoinHandle<()>> = vec![];

        for thread_id in 0..thread_count {
            let scopy = Arc::clone(&sync);
            let twg = wg.clone();
            let tstop = stop.clone();
            let texpect = expected_foo.clone();
            let tcleared = cleared.clone();
            let tnext_bar = next_bar.clone();

            thandles.push(s.spawn(move |_| {
                twg.wait();
                let mut i = 0;
                while !tstop.load(Ordering::Relaxed) {
                    i += 1;
                    eprintln!("[{:>4}] {:?}", thread_id, scopy.read_foo().clone());
                    assert_eq!(*scopy.read_foo(), (*texpect.lock().clone()), "foo value");
                    if (i % 13) == 0 {
                        eprintln!("Time to clear for {} since {} % 13 == {}", thread_id, i, i % 13);
                        // Prevent other threads from accessing foo untils we're done updating bar
                        let lock_foo = scopy.write_foo();
                        let mut wnext = tnext_bar.lock();
                        *wnext += 1;
                        scopy.set_next_bar(*wnext);
                        scopy.clear_bar();
                        assert!(!scopy.has_bar(), "bar is cleared and stays so until foo is unlocked");
                        tcleared.fetch_add(1, Ordering::SeqCst);
                        *texpect.lock() = format!("Foo with bar={}", *wnext).to_string();
                        lock_foo.clear();
                        eprintln!("Now should expect for '{}' // {}", *texpect.lock(), scopy.has_foo());
                    }
                }
                eprintln!("[{:>4}] done", thread_id);
            }));
        }

        wg.wait();
        std::thread::sleep(time::Duration::from_millis(100));
        stop.store(true, Ordering::Relaxed);
        for thandle in thandles {
            thandle.join().expect("thread join failed");
        }
        // There is a chance for two or more consecutive clears to take place before a build is invoked.
        assert!(
            cleared.load(Ordering::SeqCst) >= sync.bar_builds(),
            "there were no more builds than clears"
        );
    })
    .unwrap();
}
