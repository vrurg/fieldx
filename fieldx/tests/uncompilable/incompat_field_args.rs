use fieldx::fxstruct;

#[fxstruct]
struct Foo {
    #[fieldx(lazy, optional)]
    foo: i32,
}

fn main() {}
