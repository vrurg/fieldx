use fieldx::fxstruct;

#[fxstruct]
struct Foo {
    #[fieldx(copy, clearer)]
    foo: String,
}