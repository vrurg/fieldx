use fieldx::fxstruct;

#[fxstruct(optional)]
struct Foo {
    #[fieldx(get(as_ref), set(into))]
    foo: String,

    #[fieldx(get(as_ref), default("default".to_string()))]
    with_default: String,
}

#[fxstruct]
struct Bar {
    #[fieldx(optional, get(as_ref), set(into))]
    bar: String,
}

#[test]
fn it_is_optional() {
    let mut foo = Foo::new();
    assert_eq!(foo.foo(), None);
    foo.set_foo("manual");
    assert_eq!(foo.foo(), Some(&"manual".to_string()));
    assert_eq!(foo.with_default(), Some(&"default".to_string()));

    let mut bar = Bar::new();
    assert_eq!(bar.bar(), None);
    bar.set_bar("manual");
    assert_eq!(bar.bar(), Some(&"manual".to_string()));
}
