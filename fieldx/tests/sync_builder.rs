#![cfg(feature = "sync")]
mod foo {
    use fieldx::fxstruct;

    #[fxstruct(sync, builder)]
    #[derive(Debug)]
    pub(crate) struct Foo {
        #[fieldx(lazy, clearer)]
        foo: String,

        #[fieldx(lazy, clearer, builder(into), get(copy))]
        real: f32,

        #[fieldx(reader, get(clone), set, builder(into), default("initial".to_string()))]
        locked_bar: String,

        #[fieldx(lazy, clearer, default(Self::default_string()))]
        lazy_default: String,

        #[fieldx(inner_mut, set, get, get_mut, builder(into))]
        modifiable: String,
    }

    impl Foo {
        fn build_foo(&self) -> String {
            "це є ледачим значенням".into()
        }

        fn build_real(&self) -> f32 {
            let l: u16 = (*self.foo()).chars().count().try_into().expect("u16 value");
            l.into()
        }

        fn build_lazy_default(&self) -> String {
            "this is a lazy default".into()
        }

        fn default_string() -> String {
            "this is default string value".to_string()
        }
    }
}

#[test]
fn basics() {
    let foo = foo::Foo::builder()
        .foo("це користувацьке значення".to_string())
        .real(1u16)
        .locked_bar("custom set")
        .modifiable("from builder")
        .build()
        .unwrap();

    assert_eq!(foo.clear_real(), Some(1f32), "real was set manually");
    assert_eq!(foo.real(), 25f32, "real was set lazily");
    assert_eq!(
        foo.clear_foo(),
        Some("це користувацьке значення".to_string()),
        "foo was set manually"
    );
    assert_eq!(
        *foo.foo(),
        "це є ледачим значенням".to_string(),
        "foo re-initialized lazily"
    );
    foo.clear_real();
    assert_eq!(foo.real(), 22f32, "real re-initialized lazily from new foo value");

    assert_eq!(*foo.read_locked_bar(), "custom set", "locked_bar was set manually");
    assert_eq!(foo.locked_bar(), "custom set", "locked_bar accessor");
    foo.set_locked_bar("with setter".to_string());
    assert_eq!(foo.locked_bar(), "with setter", "locked_bar setter works");

    assert_eq!(*foo.modifiable(), "from builder".to_string());
    let old = foo.set_modifiable("set manually".to_string());
    assert_eq!(old, "from builder".to_string());
    assert_eq!(*foo.modifiable(), "set manually".to_string());
    *foo.modifiable_mut() = "via get_mut".to_string();
    assert_eq!(*foo.modifiable(), "via get_mut".to_string());
}

#[test]
fn empties() {
    let foo = foo::Foo::builder().modifiable("non-skippable").build().unwrap();

    assert_eq!(
        *foo.foo(),
        "це є ледачим значенням".to_string(),
        "when no value from builder foo gets built"
    );

    assert_eq!(foo.real(), 22f32, "when no manual real is built from foo");

    assert_eq!(
        foo.locked_bar(),
        "initial",
        "when no value from builder locked_bar gets its default value"
    );

    assert_eq!(
        *foo.lazy_default(),
        "this is default string value",
        "lazy field gets a default if not set"
    );

    let foo = foo::Foo::builder()
        .lazy_default("non-lazy, non-default".to_string())
        .modifiable("non-skippable")
        .build()
        .expect("Plain instance");

    assert_eq!(
        *foo.lazy_default(),
        "non-lazy, non-default",
        "lazy field set manually, default is ignored"
    );

    foo.clear_lazy_default();
    assert_eq!(
        *foo.lazy_default(),
        "this is a lazy default",
        "lazy field gets set by its builder when cleared"
    );
}
