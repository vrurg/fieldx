use fieldx::fxstruct;
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

trait Newish {
    fn new() -> Self;
}

#[fxstruct(builder, into)]
#[derive(Debug)]
struct NonSync<'a, 'b, T>
where
    T: Display + Debug + Default + Newish,
    'b: 'a,
{
    #[fieldx(lazy, clearer)]
    foo: T,

    #[fieldx(lazy, clearer)]
    bar: T,

    _p1: PhantomData<&'a T>,
    _p2: PhantomData<&'b T>,
}

impl<'a, 'b, T> NonSync<'a, 'b, T>
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
    let mut nonsync = NonSync::<String>::builder()
        .foo("Foo manual")
        .bar("Bar manual")
        .build()
        .expect("NonSync instance");

    assert_eq!(nonsync.bar(), &"Bar manual".to_string(), "manually set value for bar");
    assert_eq!(
        nonsync.clear_foo(),
        Some("Foo manual".to_string()),
        "foo was set manually"
    );
    assert_eq!(
        nonsync.foo(),
        &String::from("my default string"),
        "foo lazily set to our override"
    );
    assert_eq!(
        nonsync.clear_bar(),
        Some("Bar manual".to_string()),
        "bar was set manually"
    );
    assert_eq!(
        nonsync.bar(),
        &String::from("my default string"),
        "bar lazily set to our override"
    );
}
