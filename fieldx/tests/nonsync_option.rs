use fieldx::fxstruct;

#[fxstruct(into)]
struct Foo {
    #[fieldx(get(clone), set, private, predicate)]
    foo: Option<String>,
}

#[test]
fn optional_supported() {
    let mut foo = Foo::new();

    assert!(!foo.has_foo(), "Initially unset");

    foo.set_foo(Some("The Answer".to_string()));
    assert_eq!(foo.foo(), Some(Some("The Answer".to_string())));
    foo.set_foo(None);
    assert_eq!(foo.foo(), Some(None));
    assert!(foo.has_foo(), "None in the field doesn't mean it's not set")
}
