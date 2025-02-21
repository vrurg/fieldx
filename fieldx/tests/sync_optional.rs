#![cfg(feature = "sync")]
use fieldx::fxstruct;

#[fxstruct(sync, optional)]
struct Foo {
    #[fieldx(get(as_ref), set(into), reader(off))]
    foo: String,

    #[fieldx(lazy, get(copy), set, clearer)]
    non_optional: i32,
}

impl Foo {
    fn build_non_optional(&self) -> i32 {
        42
    }
}

#[fxstruct(sync)]
struct Bar {
    #[fieldx(optional, get, set(into), reader(off))]
    bar: String,
}

#[test]
fn it_is_optional() {
    let foo = Foo::new();
    assert_eq!(*foo.foo(), None);
    foo.set_foo("manual");
    assert_eq!(*foo.foo(), Some("manual".to_string()));

    let bar = Bar::new();
    assert_eq!(*bar.bar(), None);
    bar.set_bar("manual");
    assert_eq!(*bar.bar(), Some("manual".to_string()));
}

#[test]
fn non_optional() {
    let foo = Foo::new();
    assert_eq!(foo.non_optional(), 42);
    foo.clear_non_optional();
    assert_eq!(foo.non_optional(), 42);
    foo.set_non_optional(666);
    assert_eq!(foo.non_optional(), 666);
    foo.clear_non_optional();
    assert_eq!(foo.non_optional(), 42);
}
