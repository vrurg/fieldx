use fieldx::fxstruct;
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

trait Newish {
    fn new() -> Self;
}

#[fxstruct(sync, builder)]
#[derive(Debug)]
struct Foo<'a, 'b, T>
where
    T: Display + Debug + Default + Send + Sync + Newish,
    'b: 'a,
    'a: 'static,
{
    #[fieldx(lazy, clearer)]
    foo: T,

    #[fieldx(lazy, clearer, into)]
    real: f32,

    _p1: PhantomData<&'a T>,
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
        let l: u16 = format!("{}", (*self.read_foo()).as_ref().unwrap())
            .chars()
            .count()
            .try_into()
            .expect("u16 value");
        l.try_into().expect("f32 value")
    }
}

impl Newish for String {
    fn new() -> Self {
        "my default string".into()
    }
}

#[test]
fn basics() {
    let foo = Foo::<String>::builder()
        .foo("це користувацьке значення".into())
        .real(12u16)
        .build()
        .unwrap();

    assert_eq!(foo.clear_real(), Some(12.0f32), "real was set manually using Into");
    assert_eq!(*foo.read_real(), Some(25.0f32), "real is set lazily");
    assert_eq!(foo.clear_foo(), Some("це користувацьке значення".to_string()), "foo is set manually");
    assert_eq!(*foo.read_foo(), Some("my default string".to_string()), "foo is set lazily");
    foo.clear_real();
    assert_eq!(*foo.read_real(), Some(17.0f32), "real is re-initialized lazily using new foo value");
}
