use fieldx::fxstruct;
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

pub trait Newish {
    fn new() -> Self;
}

#[fxstruct(builder, into)]
#[derive(Debug)]
pub struct Plain<'a, 'b, T>
where
    T: Display + Debug + Default + Newish,
    'b: 'a,
{
    #[fieldx(lazy, clearer)]
    foo: T,

    #[fieldx(lazy, clearer)]
    bar: T,

    #[fieldx(inner_mut, get)]
    modifiable: T,

    #[fieldx(builder(off))]
    _p1: PhantomData<&'a T>,
    #[fieldx(default)]
    _p2: PhantomData<&'b T>,
}

impl<'a, 'b, T> Plain<'a, 'b, T>
where
    T: Display + Debug + Default + Newish,
    'b: 'a,
{
    fn build_foo(&self) -> T {
        T::new()
    }

    fn build_bar(&self) -> T {
        T::new()
    }
}

impl Newish for String {
    fn new() -> Self {
        "my default string".into()
    }
}

#[test]
fn basic() {
    let mut plain = Plain::<String>::builder()
        .foo("Foo manual")
        .bar("Bar manual")
        .modifiable("from builder".to_string())
        .build()
        .expect("Plain instance");

    assert_eq!(*plain.modifiable(), "from builder".to_string());
    assert_eq!(plain.bar(), &"Bar manual".to_string(), "manually set value for bar");
    assert_eq!(
        plain.clear_foo(),
        Some("Foo manual".to_string()),
        "foo was set manually"
    );
    assert_eq!(
        plain.foo(),
        &String::from("my default string"),
        "foo lazily set to our override"
    );
    assert_eq!(
        plain.clear_bar(),
        Some("Bar manual".to_string()),
        "bar was set manually"
    );
    assert_eq!(
        plain.bar(),
        &String::from("my default string"),
        "bar lazily set to our override"
    );
}
