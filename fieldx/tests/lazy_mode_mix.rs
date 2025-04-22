#![cfg(all(feature = "sync", feature = "async"))]
#![allow(clippy::approx_constant)]

use fieldx::fxstruct;

// Ensure that the correct mode is chosen for a field.
// The primary test purpose is to check compilability.
#[fxstruct(r#async)]
struct Foo {
    #[fieldx(lazy, get(copy))]
    async_lazish: f32,
    #[fieldx(sync, lazy, get(copy))]
    sync_lazish:  i32,
}

impl Foo {
    async fn build_async_lazish(&self) -> f32 {
        3.1415926
    }

    fn build_sync_lazish(&self) -> i32 {
        42
    }
}

#[fxstruct(sync)]
struct Bar {
    #[fieldx(lazy, get(copy))]
    sync_lazish:  i32,
    #[fieldx(mode(plain), lazy, get(clone))]
    plain_lazish: String,
}

impl Bar {
    fn build_sync_lazish(&self) -> i32 {
        42
    }

    fn build_plain_lazish(&self) -> String {
        "forty-two".to_string()
    }
}

#[fxstruct]
struct Baz {
    #[fieldx(lazy, get(copy))]
    plain_lazish: i32,

    #[fieldx(sync, lazy, get(clone))]
    sync_lazish: String,
}

impl Baz {
    fn build_plain_lazish(&self) -> i32 {
        42
    }

    fn build_sync_lazish(&self) -> String {
        "forty-two".to_string()
    }
}

#[tokio::test]
async fn do_something() {
    let foo = Foo::new();
    assert_eq!(foo.async_lazish().await, 3.1415926);
    assert_eq!(foo.sync_lazish(), 42);

    let bar = Bar::new();
    assert_eq!(bar.sync_lazish(), 42);
    assert_eq!(bar.plain_lazish(), "forty-two");

    let baz = Baz::new();
    assert_eq!(baz.plain_lazish(), 42);
    assert_eq!(baz.sync_lazish(), "forty-two");
}
