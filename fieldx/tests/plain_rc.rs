use fieldx::fxstruct;
use std::rc::{Rc, Weak};

#[derive(Debug)]
#[fxstruct(rc(vis(pub)))]
struct Bar {
    #[fieldx(get(copy), attributes_fn(allow(dead_code)))]
    id: usize,
}

#[derive(Debug)]
#[fxstruct(rc, builder)]
struct Foo {
    #[fieldx(lazy, get, builder)]
    bar: Rc<Bar>,
}

impl Foo {
    fn build_bar(self: Rc<Self>) -> Rc<Bar> {
        Bar::new()
    }
}

#[test]
fn type_check() {
    let foo: Rc<Foo> = Foo::new();
    assert_eq!(Rc::weak_count(&foo), 1, "Implicit Weak reference is there");
    let bar: &Rc<Bar> = foo.bar();
    let _bar_copy: Weak<Bar> = bar.myself_downgrade();
}

#[test]
fn builder() {
    let foo: Rc<Foo> = Foo::builder()
        .bar(Bar::new())
        .build()
        .expect("There was an error producing Foo instance");
    let _foo_copy = Rc::clone(&foo);
    assert_eq!(Rc::strong_count(&foo), 2);
}
