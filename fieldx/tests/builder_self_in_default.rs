//! Make sure that `Self::` in `default` works in builder. Internally this is guaranteed by translating the `Self` into
//! `<Foo>` type, so it should work everywhere where `Foo` is visible.

#[cfg(feature = "sync")]
mod sync {
    use fieldx::fxstruct;

    #[fxstruct(sync, get, builder)]
    pub struct Foo {
        #[fieldx(default(Self::default_string()))]
        bar: String,
    }

    impl Foo {
        fn default_string() -> String {
            "default value".to_string()
        }
    }
}

mod plain {
    use fieldx::fxstruct;

    #[fxstruct(get, builder)]
    pub struct Foo {
        #[fieldx(default(Self::default_string()))]
        bar: String,
    }

    impl Foo {
        fn default_string() -> String {
            "default value".to_string()
        }
    }
}

#[cfg(feature = "sync")]
#[test]
fn test_sync_foo_builder() {
    let foo = sync::Foo::builder().build().unwrap();
    assert_eq!(foo.bar(), "default value");
}

#[test]
fn test_plain_foo_builder() {
    let foo = plain::Foo::builder().build().unwrap();
    assert_eq!(foo.bar(), "default value");
}
