use fieldx::fxstruct;

#[fxstruct(sync)]
struct TT {
    #[fieldx(lazy, reader(attributes_fn(deny(dead_code))))]
    tt: String,
}

impl TT {
    fn build_tt(&self) -> String { "whatever".into() }
}

fn main() {}