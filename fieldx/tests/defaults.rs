use fieldx::fxstruct;

#[fxstruct(default)]
struct Foo {
    #[fieldx(get(copy))]
    v: i32,
}

#[test]
fn with_default() {
    let foo = Foo::new();
    assert_eq!(foo.v(), 0);
}
