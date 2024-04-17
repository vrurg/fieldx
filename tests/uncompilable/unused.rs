use fieldx::fxstruct;

#[fxstruct(builder)]
struct Unused {
    #[fieldx(
        lazy,
        // dead_code on builder foo() method must cause compilation failure.
        builder(attributes_fn(deny(dead_code)))
    )]
    foo: String,
}

impl Unused {
    fn build_foo(&self) -> String {
        "some".to_string()
    }
}

fn main() {}