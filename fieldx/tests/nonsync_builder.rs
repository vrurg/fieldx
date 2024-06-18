// With this `deny` we make sure that attributes_impl is actually applied.
#![deny(dead_code)]
use fieldx::fxstruct;

#[fxstruct(builder(public(crate), attributes(derive(Debug))), attributes_impl(allow(dead_code)))]
#[derive(Debug)]
struct NonSync {
    #[fieldx(lazy, clearer, predicate, rename("dummy"))]
    foo:    String,
    #[fieldx(lazy, private, predicate, clearer, set, builder(attributes_fn(allow(dead_code))))]
    bar:    i32,
    #[fieldx(default(3.1415926))]
    pub pi: f32,

    // Let's try a charged but not lazy field
    #[fieldx(
        clearer,
        predicate,
        set,
        default("bazzification"),
        builder(attributes_fn(allow(dead_code)))
    )]
    baz: String,

    #[fieldx(clearer, predicate, set, builder(attributes_fn(allow(dead_code))))]
    fubar: f32,

    #[fieldx(lazy, clearer, default(Self::default_string()))]
    lazy_default: String,
}

impl NonSync {
    fn build_dummy(&self) -> String {
        format!("this is foo with bar={}", self.bar()).to_string()
    }

    fn build_bar(&self) -> i32 {
        42
    }

    fn build_lazy_default(&self) -> String {
        "this is a lazy default".into()
    }

    fn default_string() -> String {
        "this is default string value".to_string()
    }
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

    assert_eq!(
        nonsync.lazy_default(),
        "this is default string value",
        "lazy field gets a default if not set"
    );

    let mut nonsync = NonSync::builder()
        .lazy_default("non-lazy, non-default".to_string())
        .build()
        .expect("NonSync instance");
    assert_eq!(
        nonsync.lazy_default(),
        "non-lazy, non-default",
        "lazy field set manually, default is ignored"
    );
    nonsync.clear_lazy_default();
    assert_eq!(
        nonsync.lazy_default(),
        "this is a lazy default",
        "lazy field gets set by its builder when cleared"
    );
}
