#![cfg(feature = "serde")]
// Make sure shadow struct is not produced if both serialize and deserialize arguments of struct `serde` argument are
// disabled.
use fieldx::fxstruct;
use serde::{Deserialize, Serialize};
use serde_json;

#[fxstruct(sync, serde(serialize(off)))]
struct Foo {
    v: &'static str,
}

fn main() {
    let f = Foo { v: "whatever" };
    let _json = serde_json::to_string(&foo);
}
