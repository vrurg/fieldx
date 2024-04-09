mod foo {
    use fieldx::fxstruct;

    #[fxstruct(sync, builder)]
    #[derive(Debug)]
    pub(crate) struct Foo {
        #[fieldx(lazy, clearer)]
        foo: String,

        #[fieldx(lazy, clearer, into)]
        real: f32,

        #[fieldx(reader, writer, get, set, into, default = "initial")]
        locked_bar: String,
    }

    impl Foo {
        fn build_foo(&self) -> String {
            "це є ледачим значенням".into()
        }

        fn build_real(&self) -> f32 {
            let l: u16 = (*self.read_foo()).chars().count().try_into().expect("u32 value");
            l.try_into().expect("f32 value")
        }
    }
}

#[test]
fn basics() {
    let foo = foo::Foo::builder()
        .foo("це користувацьке значення".to_string())
        .real(1u16)
        .locked_bar("custom set")
        .build()
        .unwrap();

    assert_eq!(foo.clear_real(), Some(1f32), "real was set manually");
    assert_eq!(*foo.read_real(), 25f32, "real was set lazily");
    assert_eq!(
        foo.clear_foo(),
        Some("це користувацьке значення".to_string()),
        "foo was set manually"
    );
    assert_eq!(
        *foo.read_foo(),
        "це є ледачим значенням".to_string(),
        "foo re-initialized lazily"
    );
    foo.clear_real();
    assert_eq!(*foo.read_real(), 22f32, "real re-initialized lazily from new foo value");

    assert_eq!(*foo.read_locked_bar(), "custom set", "locked_bar was set manually");
    assert_eq!(foo.locked_bar(), "custom set", "locked_bar accessor");
    foo.set_locked_bar("with setter".to_string());
    assert_eq!(foo.locked_bar(), "with setter", "locked_bar setter works");
}
