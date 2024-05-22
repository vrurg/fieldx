use fieldx::fxstruct;

#[fxstruct(sync, optional)]
struct Foo {
    #[fieldx(get(as_ref),set(into), reader(off))]
    foo: String,
}

#[fxstruct(sync)]
struct Bar {
    #[fieldx(optional, get, set(into), reader(off))]
    bar: String,
}

#[test]
fn it_is_optional() {
    let foo = Foo::new();
    assert_eq!(foo.foo(), None);
    foo.set_foo("manual");
    assert_eq!(foo.foo(), Some("manual".to_string()));

    let bar = Bar::new();
    assert_eq!(bar.bar(), None);
    bar.set_bar("manual");
    assert_eq!(bar.bar(), Some("manual".to_string()));
}