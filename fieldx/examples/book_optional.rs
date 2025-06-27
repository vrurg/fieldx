#![allow(unused)]

use fieldx::fxstruct;

// ANCHOR: decl
#[fxstruct]
struct Foo {
    #[fieldx(clearer, predicate, set)]
    value: String,
}
// ANCHOR_END: decl

#[test]
#[rustfmt::skip]
fn test() {
// ANCHOR: usage
let mut foo = Foo::new();
assert!(!foo.has_value());
foo.set_value("Hello, world!".to_string());
assert!(foo.has_value());
let old_value = foo.clear_value();
assert!(!foo.has_value());
assert_eq!(old_value, Some("Hello, world!".to_string()));
// ANCHOR_END: usage
}

// ANCHOR: as_ref_decl
#[fxstruct]
struct Bar {
    #[fieldx(optional, get(as_ref), set)]
    value: String,
}
// ANCHOR_END: as_ref_decl

#[test]
#[rustfmt::skip]
fn test_as_ref() {
// ANCHOR: as_ref_usage
let mut bar = Bar::new();
bar.set_value("Привіт, світ!".to_string());
assert_eq!(bar.value(), Some(&"Привіт, світ!".to_string()));
// ANCHOR_END: as_ref_usage
}

fn main() {}
