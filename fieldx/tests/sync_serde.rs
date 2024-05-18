#![cfg(feature = "serde")]
use fieldx::fxstruct;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Default, Clone, Debug, PartialEq)]
struct Bar {
    v: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct Baz {
    cnt: u32,
}

#[fxstruct(
    sync,
    builder(attributes_impl(allow(dead_code))),
    serde(default, shadow_name("FooDup"))
)]
#[derive(Clone, Debug)]
struct Foo {
    #[fieldx(lazy, serde(off))]
    bar: Bar,

    #[fieldx(lazy)]
    baz: Baz,

    #[fieldx(
        lazy,
        get(copy),
        clearer,
        default(Self::init_count()),
        serde(deserialize(off), forward_attrs(a1, b2))
    )]
    count: i32,

    #[fieldx(lazy, get(copy), serde)]
    pi: f64,

    #[fieldx(predicate, clearer, set, get(copy))]
    opt: u64,

    #[fieldx(default(-1122.3344))]
    simple: f64,

    // Of the two defaults serde wins when deserializing.
    #[fieldx(serde( default(-987.654) ), default(12.34), rename("sssimple"))]
    simple2: f64,
}

impl Foo {
    fn build_bar(&self) -> Bar {
        Bar {
            v: "from lazy".to_string(),
        }
    }

    fn build_baz(&self) -> Baz {
        Baz { cnt: 123 }
    }

    fn build_count(&self) -> i32 {
        1
    }

    fn build_pi(&self) -> f64 {
        std::f64::consts::PI
    }

    fn init_count() -> i32 {
        13
    }
}

#[test]
fn basics() {
    let foo = Foo::builder().simple(666.13).build().expect("Foo builder failed");

    let json = serde_json::to_string(&foo).expect("Foo serialization failure");

    assert_eq!(
        json, r#"{"baz":{"cnt":123},"count":13,"pi":3.141592653589793,"opt":null,"simple":666.13,"sssimple":12.34}"#,
        "serialized"
    );

    foo.set_opt(12);
    foo.clear_count();

    let json = serde_json::to_string(&foo).expect("Foo serialization failure");

    assert_eq!(
        json, r#"{"baz":{"cnt":123},"count":1,"pi":3.141592653589793,"opt":12,"simple":666.13,"sssimple":12.34}"#,
        "serialized after changes"
    );

    let json_src = r#"{"baz":{"cnt":9876},"count":112233,"pi":3.141,"opt":null,"simple":-13.666,"sssimple":999.111}"#;
    let foo_de = serde_json::from_str::<Foo>(&json_src).expect("Foo deserialization failure");

    assert_eq!(
        *foo_de.read_bar(),
        Bar {
            v: "from lazy".to_string(),
        },
        "bar is not deserializable"
    );
    assert_eq!(
        *foo_de.read_baz(),
        Baz { cnt: 9876 },
        "a lazy field with struct got deserialized"
    );
    assert_eq!(
        foo_de.count(),
        13,
        "a lazy non-deserializable u32 field gets its default after deserialization"
    );
    assert_eq!(foo_de.pi(), 3.141, "a lazy f64 field – deserialized");
    assert_eq!(
        foo_de.opt(),
        None,
        "an optional u64 field deserializes to None from JSON's 'null'"
    );
    assert_eq!(foo_de.simple, -13.666, "a plain field – deserialized");
    assert_eq!(foo_de.simple2, 999.111, "a renamed plain field – deserialized");

    let json_src = r#"{"baz":{"cnt":9876},"pi":3.141,"opt":31415926}"#;
    let foo_de = serde_json::from_str::<Foo>(&json_src).expect("Foo deserialization failure");

    // eprintln!("{:#?}", foo_de);

    assert_eq!(foo_de.opt(), Some(31415926), "an optional u64 field - deserialized");
    assert_eq!(
        foo_de.simple, -1122.3344,
        "a plain field gets its default after deserialization if missing from JSON"
    );
    assert_eq!(
        foo_de.simple2, -987.654,
        "a plain field gets its serde default after deserialization if missing from JSON"
    );
}
