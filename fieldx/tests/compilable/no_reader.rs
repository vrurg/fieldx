#[cfg(feature = "sync")]
mod inner {
    // This is an "answer" to ../uncompilable/unused_reader.rs. It compiles because of reader(off)
    use fieldx::fxstruct;

    #[fxstruct(sync)]
    struct TT {
        #[fieldx(lazy, reader(off, attributes_fn(deny(dead_code))))]
        tt: String,
    }

    impl TT {
        fn build_tt(&self) -> String {
            "whatever".into()
        }
    }
}

fn main() {}
