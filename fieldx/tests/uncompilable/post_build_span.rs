use fieldx::fxstruct;

#[fxstruct(builder(post_build))]
struct Foo {
    #[fieldx(get(clone), set, into)]
    foo: String,
}

impl Foo {
    fn post_build(&self) {}
}

mod my {
    use fieldx::error::FieldXError;
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("builder error: {0}")]
        Builder(#[from] FieldXError),
    }
}

#[fxstruct(builder(error(my::Error), post_build))]
struct Bar {
    bar: i32,
}

impl Bar {
    fn post_build(self) {}
}
