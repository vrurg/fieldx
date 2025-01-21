use fieldx::fxstruct;

#[fxstruct(default, get)]
struct Foo {
    #[fieldx(get(copy))]
    v: i32,

    #[fieldx(default = "from name/value")]
    s1: &'static str,

    #[fieldx(default("from list"))]
    s2: &'static str,

    #[fieldx(default(off, "must not be used".to_string()))]
    s3: String,

    #[fieldx(default("from method call".to_string()))]
    s4: String,
}

#[test]
fn with_default() {
    let foo = Foo::new();
    assert_eq!(foo.v(), 0);

    assert_eq!(foo.s1(), &"from name/value");
    assert_eq!(foo.s2(), &"from list");
    assert_eq!(foo.s3(), &String::default());
    assert_eq!(foo.s4(), &"from method call".to_string());
}
