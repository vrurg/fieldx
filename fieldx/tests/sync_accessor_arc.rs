#![cfg(feature = "sync")]
use fieldx::fxstruct;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy)]
struct BarCopy {
    x: i32,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
struct BarClone {
    s: String,
}

#[fxstruct(sync, rc, builder)]
#[derive(Debug)]
struct Foo {
    #[fieldx(get, copy)]
    bar_copy: BarCopy,

    #[fieldx(get(clone))]
    bar_clone: BarClone,

    #[fieldx(lazy, get, copy, builder(off))]
    lazy_bar_copy: BarCopy,

    #[fieldx(lazy, get(clone), builder(off))]
    lazy_bar_clone: BarClone,

    // Make sure it is possible to lock-protect a field
    #[fieldx(reader, writer("write_lb"), default(String::from("protected")), builder(off))]
    locked_bar: String,

    #[fieldx(lazy, inner_mut, get_mut, builder(off))]
    queue: Vec<String>,
}

impl Foo {
    fn for_test() -> Arc<Self> {
        Self::builder()
            .bar_copy(BarCopy { x: 15 })
            .bar_clone(BarClone {
                s: "statically set".into(),
            })
            .build()
            .expect("Failed to build test instance of Foo")
    }

    fn build_lazy_bar_copy(&self) -> BarCopy {
        BarCopy { x: -15 }
    }

    fn build_lazy_bar_clone(&self) -> BarClone {
        BarClone {
            s: "lazily created".into(),
        }
    }

    fn build_queue(&self) -> Vec<String> {
        vec!["foo".into(), "bar".into()]
    }
}

#[test]
fn sync_accessors() {
    let foo = Foo::for_test();

    // Copy and clone accessors would return new instances on every read. Thus any two obtained values must be different
    // locations in memory.

    let bcopy1 = foo.bar_copy();
    let bcopy2 = foo.bar_copy();
    assert_ne!(&bcopy1 as *const _, &bcopy2 as *const _, "copy accessor to plain field");

    let bclone1 = foo.bar_clone();
    let bclone2 = foo.bar_clone();
    assert_ne!(
        &bclone1 as *const _, &bclone2 as *const _,
        "clone accessor to plain field"
    );

    let bcopy1 = foo.lazy_bar_copy();
    let bcopy2 = foo.lazy_bar_copy();
    assert_ne!(&bcopy1 as *const _, &bcopy2 as *const _, "copy accessor to plain field");

    let bclone1 = foo.lazy_bar_clone();
    let bclone2 = foo.lazy_bar_clone();
    assert_ne!(
        &bclone1 as *const _, &bclone2 as *const _,
        "clone accessor to plain field"
    );
}

#[test]
fn sync_locked() {
    let foo = Foo::for_test();

    assert_eq!(
        *foo.read_locked_bar(),
        "protected",
        "read-lock is supported for non-optional, non-lazy field"
    );
    *foo.write_lb() = "changed".to_string();
    assert_eq!(*foo.read_locked_bar(), "changed", "updated the field via write-lock");
}

#[test]
fn mutable() {
    let foo = Foo::for_test();

    assert_eq!(
        *foo.queue(),
        vec!["foo".to_string(), "bar".to_string()],
        "initial lazy vector value"
    );
    foo.queue_mut().push("baz".into());
    assert_eq!(
        *foo.queue(),
        vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
        "lazy vector with new elem"
    );
}
