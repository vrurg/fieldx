mod inner {
    use fieldx::fxstruct;

    #[fxstruct(sync)]
    pub struct Inner {
        #[fieldx(vis(), lazy, reader)]
        foo: u8,
    }

    impl Inner {
        fn build_foo(&self) -> u8 {
            42
        }
    }
}

fn main() {
    let inner = inner::Inner::new();
    // This must not compile, as expected.
    let foo = inner.read_foo();
}
