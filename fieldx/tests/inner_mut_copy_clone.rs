use fieldx::fxstruct;

#[fxstruct(sync, builder)]
struct Foo {
    #[fieldx(get(copy))]
    m1: u32,

    #[fieldx(get(clone))]
    m2: String,
}

#[test]
fn basic() {
    let foo = Foo::builder()
        .m1(42)
        .m2("from builder".to_string())
        .build()
        .expect("Builder failed");

    assert_eq!(foo.m1(), 42);
    assert_eq!(foo.m2(), "from builder".to_string());
}
