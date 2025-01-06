#![cfg(feature = "sync")]
use fieldx::fxstruct;

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

#[fxstruct(sync)]
#[derive(Debug)]
struct Foo {
    #[fieldx(get, copy)]
    bar_copy: BarCopy,

    #[fieldx(get(clone))]
    bar_clone: BarClone,

    #[fieldx(lazy, get, copy)]
    lazy_bar_copy: BarCopy,

    #[fieldx(lazy, get(clone))]
    lazy_bar_clone: BarClone,

    // Make sure it is possible to lock-protect a field
    #[fieldx(reader, writer("write_lb"), default("protected"))]
    locked_bar: String,

    #[fieldx(lazy, get_mut)]
    queue: Vec<String>,

    #[fieldx(get, get_mut)]
    seq: Vec<u32>,

    #[fieldx(inner_mut, get, get_mut)]
    mutable: f32,
}

impl Foo {
    fn for_test() -> Self {
        Foo {
            bar_copy: BarCopy { x: 15 },
            bar_clone: BarClone {
                s: "statically set".into(),
            },
            ..Default::default()
        }
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
    let mut foo = Foo::for_test();

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

    *foo.seq_mut() = vec![12, 13, 42, 666];
    assert_eq!(
        *foo.seq(),
        vec![12, 13, 42, 666],
        "assignment into a non-protected field"
    );
    assert_eq!(foo.seq_mut().pop().unwrap(), 666, "mutate a non-protected field");

    let foo_ro = Foo::for_test();
    *foo_ro.mutable_mut() = 42.21;
    assert_eq!(*foo_ro.mutable(), 42.21, "write-lock on a non-optional field");
}
