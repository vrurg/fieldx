
mod inner {
    use fieldx::fxstruct;

    #[fxstruct(sync)]
    pub struct Inner {
        #[fieldx(private, lazy, accessor)]
        foo: u8,
    }

    impl Inner {
        fn build_foo(&self) -> u8 { 42 }
    }
}

fn main() {
    let inner = inner::Inner::new();
    // This must not compile, as expected.
    let foo = inner.foo();
}
