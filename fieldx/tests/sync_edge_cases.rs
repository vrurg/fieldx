#![deny(dead_code)]

mod inner {
    use fieldx::fxstruct;

    #[fxstruct(sync)]
    pub struct Foo {
        #[fieldx(get(copy), set, clearer, predicate,
            // reader + attributes_fn + deny(dead_code) are ensuring that attributes_fn are applied to the reader.
            reader, attributes_fn(allow(dead_code)))]
        v: u32,
    }
}

#[test]
fn main() {
    let foo = inner::Foo::new();

    assert!(!foo.has_v(), "field is initially unset");
    assert_eq!(foo.v(), None, "initially is None");
    foo.set_v(42);
    assert!(foo.has_v(), "field is set");
    assert_eq!(foo.v(), Some(42), "is 42");
    foo.clear_v();
    assert_eq!(foo.v(), None, "cleared");
}
