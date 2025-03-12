use fieldx::fxstruct;

// It's OK for this test just to compile.

#[fxstruct]
struct Foo {
    #[allow(unused)]
    f1: u32,
    #[fieldx(builder)]
    f2: u32,
}

#[test]
fn builder() {
    let foo = Foo::builder().f2(42).build().unwrap();
    assert_eq!(foo.f2, 42);
}
