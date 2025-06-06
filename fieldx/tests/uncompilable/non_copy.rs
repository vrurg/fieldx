use fieldx::fxstruct;

#[fxstruct]
struct Foo {
    #[fieldx(copy, get, clearer)]
    foo: String,
}

#[cfg(not(test))]
fn main() {}
