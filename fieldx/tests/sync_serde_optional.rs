#![cfg(all(feature = "sync", feature = "serde"))]
use fieldx::fxstruct;
use serde::Deserialize;
use serde::Serialize;

#[fxstruct(sync, builder(attributes_impl(allow(dead_code))), serde(default))]
#[derive(Clone, Debug)]
struct Foo {
    #[fieldx(lock, optional, into, get(clone))]
    maybe_name: String,

    #[fieldx(into, get(clone))]
    mandatory: String,
}

#[test]
fn no_value() {
    let foo = Foo::builder().mandatory("always have this").build().unwrap();

    assert!(foo.maybe_name().is_none());

    let json = serde_json::to_string(&foo).expect("Foo serialization failure");

    assert_eq!(
        json, r#"{"maybe_name":null,"mandatory":"always have this"}"#,
        "serialized"
    );

    let foo: Foo = serde_json::from_str(r#"{"maybe_name":null,"mandatory":"is the only defined"}"#).unwrap();

    assert!(foo.maybe_name().is_none());
    assert_eq!(foo.mandatory(), "is the only defined".to_string());
}

#[test]
fn with_value() {
    let foo = Foo::builder()
        .maybe_name("some name")
        .mandatory("always have this")
        .build()
        .unwrap();

    assert!(foo.maybe_name().is_some());

    let json = serde_json::to_string(&foo).expect("Foo serialization failure");

    assert_eq!(
        json, r#"{"maybe_name":"some name","mandatory":"always have this"}"#,
        "serialized"
    );

    let foo: Foo = serde_json::from_str(r#"{"maybe_name":"there is a name","mandatory":"is also defined"}"#).unwrap();

    assert_eq!(foo.maybe_name(), Some("there is a name".to_string()));
    assert_eq!(foo.mandatory(), "is also defined".to_string());
}
