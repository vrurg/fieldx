use std::rc::Rc;

use fieldx::fxstruct;

#[fxstruct(builder(post_build))]
struct Foo {
    #[fieldx(default(0), builder(attributes_fn(allow(dead_code))))]
    derive: u32,
}

impl Foo {
    fn post_build(mut self) -> Self {
        assert_eq!(self.derive, 0, "initial value is 0");
        self.derive = 42;
        self
    }
}

// Make sure that post_build receives fully-initialized Self with working .myself() method.
#[fxstruct(rc, builder(post_build))]
struct Bar {
    #[fieldx(inner_mut, get(copy), set, default(0), builder(attributes_fn(allow(dead_code))))]
    derive: u32,
}

impl Bar {
    fn post_build(self: Rc<Self>) -> Rc<Self> {
        assert_eq!(self.derive(), 0, "initial value is 0");
        self.myself().expect("myself is not set").set_derive(12);
        self
    }
}

#[test]
fn builder_post_build() {
    let foo = Foo::builder().build().unwrap();
    assert_eq!(foo.derive, 42, "post-build initializer works");

    let bar = Bar::builder().build().unwrap();
    assert_eq!(bar.derive(), 12, "post-build initializer works on ref-counted struct");
}
