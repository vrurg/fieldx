#![cfg(feature = "sync")]
use fieldx::fxstruct;

#[fxstruct(sync, into)]
struct Foo {
    #[fieldx(get(clone), reader, set, private, predicate, clearer)]
    foo: Option<String>,
}

#[test]
fn optional_supported() {
    let foo = Foo::new();

    assert!(!foo.has_foo(), "Initially unset");

    foo.set_foo(Some("The Answer".to_string()));
    assert_eq!(foo.foo(), Some(Some("The Answer".to_string())));
    assert_eq!(*foo.read_foo(), Some(Some("The Answer".to_string())));
    foo.set_foo(None);
    assert_eq!(foo.foo(), Some(None));
    assert_eq!(*foo.read_foo(), Some(None));
    assert!(foo.has_foo(), "None in the field doesn't mean it's not set");

    foo.clear_foo();
    assert_eq!(foo.foo(), None);
    assert_eq!(*foo.read_foo(), None);
    assert!(!foo.has_foo(), "the field is reset");
}
