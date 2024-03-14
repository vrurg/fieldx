use fieldx::fxstruct;

#[fxstruct(sync)]
struct Foo {
    #[fieldx(accessor)]
    foo: String,
}