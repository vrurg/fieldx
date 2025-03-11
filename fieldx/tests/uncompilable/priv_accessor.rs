mod inner {
    use fieldx::fxstruct;

    #[fxstruct(get, vis(pub))]
    pub struct Foo {
        #[fieldx]
        foo_rw: f32,
        #[fieldx(get(private), predicate(off))]
        foo:    u32,
    }
}

fn main() {
    let foo = inner::Foo::new();
    let _r = foo.foo_rw();
    // Must report error here
    let _r = foo.foo();
}
