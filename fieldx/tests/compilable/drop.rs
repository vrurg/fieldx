#![allow(unused)]

mod plain {
    use fieldx::fxstruct;

    #[fxstruct]
    struct Foo {
        #[fieldx(lazy, get(copy))]
        foo: bool,
    }

    impl Foo {
        fn build_foo(&self) -> bool {
            false
        }
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            if self.foo() {
                panic!("foo is true");
            }
        }
    }
}

#[cfg(feature = "sync")]
mod sync {
    use fieldx::fxstruct;

    #[fxstruct(sync)]
    struct Foo {
        #[fieldx(lazy, get(copy))]
        foo: bool,
    }

    impl Foo {
        fn build_foo(&self) -> bool {
            false
        }
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            if self.foo() {
                panic!("foo is true");
            }
        }
    }
}

#[cfg(feature = "async")]
mod r#async {
    use fieldx::fxstruct;

    #[fxstruct(r#async)]
    struct Foo {
        #[fieldx(lazy, get(copy))]
        foo: bool,
    }

    impl Foo {
        async fn build_foo(&self) -> bool {
            false
        }
    }

    impl Drop for Foo {
        fn drop(&mut self) {}
    }
}

#[cfg(feature = "serde")]
mod serde {
    use fieldx::fxstruct;
    use serde::Deserialize;
    use serde::Serialize;

    #[fxstruct(serde(default, serialize, deserialize))]
    #[derive(Clone)]
    struct Foo {
        #[fieldx(lazy, get(copy))]
        foo: bool,
    }

    impl Foo {
        fn build_foo(&self) -> bool {
            false
        }
    }

    impl Drop for Foo {
        fn drop(&mut self) {}
    }
}

fn main() {}
