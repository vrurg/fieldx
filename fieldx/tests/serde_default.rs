#![cfg(all(feature = "serde", feature = "sync"))]
use fieldx::fxstruct;
use serde::Deserialize;
use serde::Serialize;

#[fxstruct(sync, get, serde(default(Self::serde_default().into())))]
#[derive(Clone)]
struct Foo {
    f1: String,
    f2: String,
}

impl Foo {
    fn serde_default() -> Self {
        Self {
            f1: "f1 default".to_string(),
            f2: "f2 default".to_string(),
        }
    }
}

#[fxstruct(sync, get, serde(default("Self::serde_default")))]
#[derive(Clone)]
struct Bar {
    f1: String,
    f2: String,
}

impl Bar {
    fn serde_default() -> __BarShadow {
        Self {
            f1: "f1 default".to_string(),
            f2: "f2 default".to_string(),
        }
        .into()
    }
}

#[fxstruct(sync, get, serde(shadow_name("BazDup"), default(Self::serde_default().into())))]
#[derive(Clone)]
struct Baz {
    f1: String,
    f2: String,
}

impl Baz {
    fn serde_default() -> Fubar {
        Fubar {
            postfix: "from fubar".into(),
        }
    }
}

struct Fubar {
    postfix: String,
}

impl From<Fubar> for BazDup {
    fn from(value: Fubar) -> Self {
        Self {
            f1: format!("f1 {}", value.postfix),
            f2: format!("f2 {}", value.postfix),
        }
    }
}

#[test]
fn missing_key_meta() {
    let json_src = r#"{"f2": "f2 json"}"#;
    let foo_de = serde_json::from_str::<Foo>(json_src).expect("Foo deserialization failure");
    assert_eq!(*foo_de.f1(), "f1 default".to_string());
    assert_eq!(*foo_de.f2(), "f2 json".to_string());

    let json_src = r#"{"f2": "f2 json", "f1": "f1 json"}"#;
    let foo_de = serde_json::from_str::<Foo>(json_src).expect("Foo deserialization failure");
    assert_eq!(*foo_de.f1(), "f1 json".to_string());
}

#[test]
fn missing_key_str() {
    let json_src = r#"{"f2": "f2 json"}"#;
    let foo_de = serde_json::from_str::<Bar>(json_src).expect("Bar deserialization failure");
    assert_eq!(*foo_de.f1(), "f1 default".to_string());
    assert_eq!(*foo_de.f2(), "f2 json".to_string());

    let json_src = r#"{"f2": "f2 json", "f1": "f1 json"}"#;
    let foo_de = serde_json::from_str::<Bar>(json_src).expect("Bar deserialization failure");
    assert_eq!(*foo_de.f1(), "f1 json".to_string());
}

#[test]
fn missing_with_3rd_party() {
    let json_src = r#"{"f1": "f1 json"}"#;
    let foo_de = serde_json::from_str::<Baz>(json_src).expect("Bar deserialization failure");
    assert_eq!(*foo_de.f1(), "f1 json".to_string());
    assert_eq!(*foo_de.f2(), "f2 from fubar".to_string());

    let json_src = r#"{"f2": "f2 json", "f1": "f1 json"}"#;
    let foo_de = serde_json::from_str::<Baz>(json_src).expect("Bar deserialization failure");
    assert_eq!(*foo_de.f1(), "f1 json".to_string());
}
