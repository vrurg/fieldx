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

#[fxstruct(sync)]
#[derive(Debug)]
struct Foo {
    #[fieldx(get, copy)]
    bar_copy: BarCopy,

    #[fieldx(get)]
    bar_clone: BarClone,

    #[fieldx(lazy, get, copy)]
    lazy_bar_copy: BarCopy,

    #[fieldx(lazy, get)]
    lazy_bar_clone: BarClone,
}

impl Foo {
    fn for_test() -> Arc<Self> {
        Foo {
            bar_copy: BarCopy { x: 15 },
            bar_clone: BarClone {
                s: "statically set".into(),
            },
            ..Default::default()
        }
        .__fieldx_init()
    }

    fn build_lazy_bar_copy(&self) -> BarCopy {
        BarCopy { x: -15 }
    }

    fn build_lazy_bar_clone(&self) -> BarClone {
        BarClone {
            s: "lazily created".into(),
        }
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
