use fieldx::fxstruct;

#[fxstruct(builder)]
struct Unused {
    #[fieldx(
        // dead_code on builder foo() method must cause compilation failure.
        builder(attributes_fn(deny(dead_code)))
    )]
    foo: String,
}

fn main() {}
