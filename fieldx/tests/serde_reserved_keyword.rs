#![cfg(feature = "serde")]

use fieldx::fxstruct;
use serde::{Deserialize, Serialize};

// get() and builder() literal values are used as prefixes thus shielding us from "type" used as method names.
#[fxstruct(get("get_", public), serde(default), builder("set_"))]
#[derive(Clone)]
struct Foo {
    #[fieldx(rename("type"))]
    ty: String,
}

#[fxstruct(get(public), serde(default), builder(public, into))]
#[derive(Clone)]
struct Bar {
    // Similar to Foo, but on the field level literal values are used as actual methods names.
    #[fieldx(rename("match"), get("match_constraint"), builder("set_match_constraint"))]
    ma: String,
}

#[test]
fn basic() {
    let foo = Foo { ty: "bar".to_string() };
    assert_eq!(foo.get_type(), "bar");

    let foo = Foo::builder()
        .set_type("bar".to_string())
        .build()
        .expect("Builder of Foo failed");
    assert_eq!(foo.get_type(), "bar");

    let bar = Bar {
        ma: "pfx*sfx".to_string(),
    };
    assert_eq!(bar.match_constraint(), "pfx*sfx");

    let bar = Bar::builder()
        .set_match_constraint("pre*post")
        .build()
        .expect("Builder of Bar failed");
    assert_eq!(bar.match_constraint(), "pre*post");
}
