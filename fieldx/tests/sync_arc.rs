use fieldx::{errors::FieldXError, fxstruct};
use std::sync::{Arc, Weak};

#[fxstruct(sync, rc)]
struct Bar {
    #[fieldx(get(copy), attributes_fn(allow(dead_code)))]
    id: usize,
}

impl Bar {
    fn bar(self: &Arc<Self>) -> Weak<Self> {
        Arc::downgrade(self)
    }
}

#[fxstruct(sync, rc, builder)]
struct Foo {
    #[fieldx(get)]
    bar: Arc<Bar>,
}

#[test]
fn type_check() {
    let foo: Arc<Foo> = Foo::new();
    let bar: &Arc<Bar> = foo.bar();
    let _bar_copy: Weak<Bar> = bar.bar();
}

#[test]
fn builder() {
    let foo: Result<Arc<Foo>, FieldXError> = Foo::builder().bar(Bar::new()).build();
    let _foo_copy = Arc::clone(&foo.expect("There was an error producing Foo instance"));
}
