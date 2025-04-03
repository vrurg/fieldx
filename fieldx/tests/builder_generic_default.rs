#![cfg(feature = "serde")]

use fieldx::fxstruct;
use serde::Deserialize;
use serde::Serialize;

#[fxstruct(builder, get, serde)]
#[derive(Clone)]
struct Foo<S = String>
where
    S: AsRef<str> + Default + Clone + for<'a> From<&'a str>,
{
    #[fieldx(default(Self::default_foo()))]
    foo: S,
}

impl<S> Foo<S>
where
    S: AsRef<str> + Default + Clone + for<'a> From<&'a str>,
{
    fn default_foo() -> S {
        "default foo".into()
    }
}

#[test]
fn generic_default() {
    let foo = Foo::builder().foo("foo".to_string()).build().unwrap();
    let foo_s: &str = foo.foo();
    assert_eq!(foo_s, "foo", "default generic value is empty string");
}
