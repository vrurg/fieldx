// Make sure inner_mut doesn't break copy and clone getters

use fieldx::fxstruct;

#[cfg(feature = "sync")]
#[fxstruct(sync, builder, set, get_mut)]
struct Foo {
    #[fieldx(inner_mut, get(copy))]
    m1: u32,

    #[fieldx(inner_mut, get(clone))]
    m2: String,

    #[fieldx(lazy, inner_mut, get(copy), builder(off))]
    m3: i32,
}

#[cfg(feature = "sync")]
impl Foo {
    fn build_m3(&self) -> i32 {
        42
    }
}

#[fxstruct(builder, set, get_mut)]
struct Bar {
    #[fieldx(inner_mut, get(copy))]
    m1: u32,

    #[fieldx(inner_mut, get(clone))]
    m2: String,

    #[fieldx(lazy, inner_mut, get(copy), builder(off))]
    m3: i32,
}

impl Bar {
    fn build_m3(&self) -> i32 {
        42
    }
}

#[test]
#[cfg(feature = "sync")]
fn basic_sync() {
    let foo = Foo::builder()
        .m1(42)
        .m2("from builder".to_string())
        .build()
        .expect("Builder failed");

    assert_eq!(foo.m1(), 42);
    assert_eq!(foo.m2(), "from builder".to_string());

    foo.set_m1(12);

    *foo.m2_mut() = "from user".to_string();
    assert_eq!(foo.m2(), "from user");

    assert_eq!(foo.m3(), 42);
    *foo.m3_mut() = 12;
    assert_eq!(foo.m3(), 12);
}

#[test]
fn basic_plain() {
    let bar = Bar::builder()
        .m1(42)
        .m2("from builder".to_string())
        .build()
        .expect("Builder failed");

    assert_eq!(bar.m1(), 42);
    assert_eq!(bar.m2(), "from builder".to_string());

    *bar.m2_mut() = "from user".to_string();
    assert_eq!(bar.m2(), "from user");

    assert_eq!(bar.m3(), 42);
    *bar.m3_mut() = 12;
    assert_eq!(bar.m3(), 12);
}
