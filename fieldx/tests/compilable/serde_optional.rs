#[cfg(all(feature = "serde", feature = "sync"))]
mod inner {
    use fieldx::fxstruct;
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    #[fxstruct(sync, get, optional, serde)]
    struct Foo {
        foo: i32,
    }
}

fn main() {}
