#![cfg(feature = "serde")]
use fieldx::fxstruct;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct Bar {
    v: String,
}

#[fxstruct(sync, builder, serde(default))]
struct Foo {
    #[fieldx(lazy, serde(off))]
    bar: Bar,

    #[fieldx(lazy, default = Self::init_count(), serde(deserialize(off), forward_attrs(a1, b2)))]
    count: u32,

    #[fieldx(lazy, serde)]
    p: f64,
}

impl Foo {
    fn build_bar(&self) -> Bar {
        Bar {
            v: "from lazy".to_string(),
        }
    }

    fn build_count(&self) -> u32 {
        1
    }

    fn build_p(&self) -> f64 { std::f64::consts::PI }

    fn init_count() -> u32 {
        13
    }
 }

#[test]
fn basics() {
}