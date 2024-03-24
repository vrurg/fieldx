use fieldx::fxstruct;
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

trait Newish {
    fn new() -> Self;
}

#[fxstruct(builder,into)]
#[derive(Debug)]
struct NonSync<'a, 'b, T>
where
    T: Display + Debug + Default + Newish,
    'b: 'a,
{
    #[fieldx(lazy, clearer, builder=off)]
    foo: T,

    #[fieldx(lazy,clearer)]
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
        // .foo("Foo manual")
        .bar("Bar manual")
        .build()
        .expect("NonSync instance");
    println!("NON SYNC: {:?}", nonsync);
    println!("CLEARING FOO: {:?}", nonsync.clear_foo());
    println!("CLEARED: {}", nonsync.foo());
    nonsync.clear_bar();
    println!("CLEARED BAR: {}", nonsync.bar());
}
