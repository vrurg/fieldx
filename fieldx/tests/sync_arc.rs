use fieldx::fxstruct;
use std::sync::{Arc, Weak};

#[fxstruct(sync, rc)]
struct Bar {
    #[fieldx(get(copy))]
    id: usize,
}

impl Bar {
    fn bar(self: &Arc<Self>) -> Weak<Self> {
        Arc::downgrade(self)
    }
}

#[fxstruct(sync, rc)]
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
