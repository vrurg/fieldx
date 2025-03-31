use fieldx::fxstruct;

#[fxstruct(get(copy), builder)]
struct Foo {
    n: i32,

    #[fieldx(get(copy(off)))]
    s1: String,

    #[fieldx(get(clone))]
    s2: String,
}

#[test]
fn test_get_copy() {
    let foo = Foo::builder()
        .n(42)
        .s1("Hello".to_string())
        .s2("World".to_string())
        .build()
        .expect("Failed to build test instance of Foo");
    assert_eq!(foo.n(), 42);
    assert_eq!(foo.s1(), &String::from("Hello"));
    assert_eq!(foo.s2(), String::from("World"));
}
