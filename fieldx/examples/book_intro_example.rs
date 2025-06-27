#![allow(unused)]

// ANCHOR: decl
use fieldx::fxstruct;

#[fxstruct(lazy)]
struct Foo {
    #[fieldx(get(copy), predicate, clearer, default(42))]
    count:   usize,
    #[fieldx(predicate, clearer)]
    comment: String,

    #[fieldx(lazy(off), inner_mut, get, get_mut)]
    order: Vec<&'static str>,
}

impl Foo {
    fn build_count(&self) -> usize {
        self.order_mut().push("Building count.");
        12
    }

    fn build_comment(&self) -> String {
        self.order_mut().push("Building foo.");
        // If `count` isn't initialized yet it will be initialized lazily. during the call to the accessor method.
        format!("foo is using count: {}", self.count())
    }
}
// ANCHOR_END: decl

#[rustfmt::skip]
#[test]
fn test_main() {
// ANCHOR: main
let mut foo = Foo::new();
// No call to the accessor method has been made yet, the field remains uninitialized.
assert!(!foo.has_comment());

// The `count` field has a default value, so it is initialized.
assert!(foo.has_count());

// No builder methods have been called yet, so the order is empty.
assert!(foo.order().is_empty());

// For the first time the count is 42, the default value. The builder method for `comment` is using that.
assert_eq!(foo.comment(), "foo is using count: 42");

// Now we reset the count field to uninitialized state.
foo.clear_count();
assert!(!foo.has_count());

// `comment` is still initialized and reflects the original default value of `count`.
assert_eq!(foo.comment(), "foo is using count: 42");

// Reset `comment` to uninitialized state.
foo.clear_comment();

// Make sure it is unset.
assert!(!foo.has_comment());

// This time the `count` field will have its value from the builder method.
assert_eq!(foo.comment(), "foo is using count: 12");

// Both `comment` and `count` has values, so this call as just returns the value of `comment`.
assert_eq!(foo.comment(), "foo is using count: 12");

// Every call of `count` and `comment` builder methods are pushing their actions to the order field. At this point
// it must contain three entries:
// - one for the first call to `comment` where `count` had its default value and thus its builder wasn't involved;
// - and one for the call to `comment` after both fields was cleared where `count` was built by its builder method;
assert_eq!(foo.order().len(), 3);
assert_eq!(foo.order()[0], "Building foo.");
assert_eq!(foo.order()[1], "Building foo.");
assert_eq!(foo.order()[2], "Building count.");
// ANCHOR_END: main
}

fn main() {}
