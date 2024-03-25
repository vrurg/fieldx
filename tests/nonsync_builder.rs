use fieldx::fxstruct;

#[fxstruct(builder)]
#[derive(Debug)]
#[allow(dead_code)]
struct NonSync {
    #[fieldx(lazy, clearer, predicate, rename = "dummy")]
    foo:    String,
    #[fieldx(lazy, private, predicate, clearer, setter)]
    bar:    i32,
    #[fieldx(default = 3.1415926)]
    pub pi: f32,

    // Let's try a charged but not lazy field
    #[fieldx(clearer, predicate, setter, default = "bazzification")]
    baz: String,

    #[fieldx(clearer, predicate, setter)]
    fubar: f32,
}

impl NonSync {
    fn build_dummy(&self) -> String {
        format!("this is foo with bar={}", self.bar()).to_string()
    }

    fn build_bar(&self) -> i32 {
        42
    }
}

// This struct here is only to make sure we can have two fxstructs in the same scope.
#[allow(dead_code)]
#[fxstruct(builder)]
struct Foo {
    dummy: String,
}

#[test]
fn basic() {
    let mut nonsync = NonSync::builder()
        .dummy("as banal as it gets".into())
        .pi(-1.2)
        .build()
        .expect("NonSync instance");

    assert_eq!(nonsync.pi, -1.2, "pi set manually");
    assert_eq!(
        nonsync.clear_dummy(),
        Some("as banal as it gets".to_string()),
        "foo(dummy) was set manually"
    );
    assert_eq!(
        nonsync.dummy(),
        &"this is foo with bar=42".to_string(),
        "foo(dummy) was lazily set"
    );
    assert_eq!(
        nonsync.baz(),
        &Some("bazzification".to_string()),
        "baz is set with its default by the builder"
    );
    assert_eq!(nonsync.fubar(), &None, "fubar has no default and was not set");
}
