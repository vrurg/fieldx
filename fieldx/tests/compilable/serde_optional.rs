#![cfg(all(feature = "serde", feature = "sync"))]
use fieldx::fxstruct;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
#[fxstruct(sync, get, optional, serde)]
struct Foo {
    foo: i32,
}
