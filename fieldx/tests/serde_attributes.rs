#![cfg(feature = "serde")]

use fieldx::fxstruct;
use serde::{Deserialize, Serialize};
use serde_json;

#[fxstruct(sync, get, serde(attributes(serde(deny_unknown_fields))))]
#[derive(Clone, Debug)]
struct Foo {
    #[fieldx(reader)]
    f1: String,
    f2: String,
}

#[test]
fn missing_key_meta() {
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
