use fieldx::fxstruct;
use serde::{Deserialize, Serialize};

#[fxstruct(no_new, builder, get, serde)]
#[derive(Clone)]
struct Foo<S = String>
where
    S: AsRef<str> + Default + Clone,
{
    foo: S,
}

#[test]
fn generic_default() {
    let foo = Foo::builder().foo("foo".to_string()).build().unwrap();
    let foo_s: &str = foo.foo();
    assert_eq!(foo_s, "foo", "default generic value is empty string");
}
