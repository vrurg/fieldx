use fieldx::fxstruct;

#[fxstruct(sync, into)]
struct Foo {
    #[fieldx(get(clone), set, private, predicate)]
    foo: Option<String>,
}

#[test]
fn optional_supported() {
    let foo = Foo::new();

    foo.set_foo(Some("The Answer".to_string()));
    assert_eq!(foo.foo(), Some("The Answer".to_string()));
    assert_eq!(*foo.read_foo(), Some("The Answer".to_string()));
    foo.set_foo(None);
    assert_eq!(foo.foo(), None);
    assert!(foo.has_foo(), "None in the field doesn't mean it's not set")
}
