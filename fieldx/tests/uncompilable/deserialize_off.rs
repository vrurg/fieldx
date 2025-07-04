use fieldx::fxstruct;
use serde::Deserialize;
use serde::Serialize;
use serde_json;

#[fxstruct(sync, serde(deserialize(off)))]
#[derive(Clone)]
struct Foo {
    v: &'static str,
}

fn main() {
    let _json = serde_json::from_str::<Foo>(r#"{"v": "whatever"}"#);
}
