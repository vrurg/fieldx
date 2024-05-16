#![cfg(feature = "serde")]
// Make sure shadow struct is not produced if both serialize and deserialize arguments of struct `serde` argument are
// disabled.
use fieldx::fxstruct;

#[fxstruct(sync, serde(serialize(off), deserialize(off)))]
struct Foo {
    v: &'static str,
}

fn main() {
    let f = __FooShadow { v: "whatever" };
}
