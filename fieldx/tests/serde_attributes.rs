#![cfg(feature = "serde")]

use fieldx::fxstruct;
use serde::{Deserialize, Serialize};
use serde_json;

#[fxstruct(sync, builder(into), get, serde(attributes(serde(deny_unknown_fields))))]
#[derive(Clone, Debug)]
struct Foo {
    #[fieldx(reader)]
    f1: String,
    #[fieldx(serde(attributes(serde(skip_serializing_if = "Foo::no_empty"))))]
    f2: String,
}

impl Foo {
    fn no_empty(s: &str) -> bool {
        s.is_empty()
    }
}

#[test]
fn extra_field() {
    let json_src = r#"{"f2": "f2 json", "f3": "will cause 'oops!'"}"#;
    let foo_de = serde_json::from_str::<Foo>(&json_src);
    assert!(
        foo_de.is_err(),
        "attribute `#[serde(deny_unknown_fields)]` is applied to the shadow"
    );
    if let Err(err) = foo_de {
        assert!(err.is_data(), "the right error class received");
        assert_eq!(err.column(), 22, "error position");
    }
}

#[test]
fn skippable() {
    let foo = Foo::builder().f1("custom f1").f2("").build().unwrap();
    assert_eq!(serde_json::to_string(&foo).unwrap(), r#"{"f1":"custom f1"}"#, "field-level user attribute");
}
