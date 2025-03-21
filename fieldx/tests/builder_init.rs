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

#[test]
fn builder_post_build() {
    let foo = Foo::builder().build().unwrap();
    assert_eq!(foo.derive, 42, "post-build initializer works");
}
