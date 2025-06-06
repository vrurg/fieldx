#![cfg(feature = "sync")]
use fieldx::fxstruct;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;

trait Newish {
    fn new() -> Self;
}

#[fxstruct(no_new, default(off), sync, builder)]
#[derive(Debug)]
struct Foo<'a, 'b, T>
where
    T: Display + Debug + Default + Send + Sync + Newish,
    'b: 'a,
    'a: 'static,
{
    #[fieldx(lazy, clearer)]
    foo: T,

    #[fieldx(lazy, copy, clearer, into)]
    real: f32,

    #[fieldx(builder(off))]
    _p1: PhantomData<&'a T>,
    #[fieldx(default)]
    _p2: PhantomData<&'b T>,
}

impl<'a, 'b, T> Foo<'a, 'b, T>
where
    T: Display + Debug + Default + Send + Sync + Newish,
    'b: 'a,
{
    fn build_foo(&self) -> T {
        T::new()
    }

    fn build_real(&self) -> f32 {
        let l: u16 = format!("{}", *self.foo())
            .chars()
            .count()
            .try_into()
            .expect("u16 value");
        l.into()
    }
}

impl Newish for String {
    fn new() -> Self {
        "my default string".into()
    }
}

#[test]
fn basics() {
    let mut foo = Foo::<String>::builder()
        .foo("це користувацьке значення".into())
        .real(12u16)
        .build()
        .unwrap();

    assert_eq!(foo.clear_real(), Some(12.0f32), "real was set manually using Into");
    assert_eq!(foo.real(), 25.0f32, "real is set lazily");
    assert_eq!(
        foo.clear_foo(),
        Some("це користувацьке значення".to_string()),
        "foo is set manually"
    );
    assert_eq!(*foo.foo(), "my default string".to_string(), "foo is set lazily");
    foo.clear_real();
    assert_eq!(foo.real(), 17.0f32, "real is re-initialized lazily using new foo value");
}
