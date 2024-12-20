use fieldx::fxstruct;

#[fxstruct(builder)]
struct Foo<const N: usize = 1> {
    #[fieldx(get, default(N))]
    v: usize,
}

#[test]
fn generic_default() {
    let foo: Foo = Foo::builder().build().unwrap();
    assert_eq!(foo.v, 1, "default generic value is 1");
}

#[test]
fn generic_2() {
    let foo = Foo::<2>::builder().build().unwrap();
    assert_eq!(foo.v, 2, "generic value is 2");
}
