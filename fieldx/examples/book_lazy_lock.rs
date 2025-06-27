#![allow(unused)]
#[cfg(feature = "sync")]
mod simple {
    use fieldx::fxstruct;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Instant;

    #[derive(Debug)]
    struct Bar {
        v: &'static str,
    }

    // ANCHOR: simple_decl
    #[rustfmt::skip]
#[fxstruct(get, builder(into))]
struct Book {
    title:     String,
    author:    String,
    #[fieldx(get(copy), builder(into(off)))]
    year:      u32,
    #[fieldx(lazy, writer, get, predicate)]
    location:  String,
}

    impl Book {
        fn build_location(&self) -> String {
            String::from("unknown")
        }
    }
    // ANCHOR_END: simple_decl

    #[rustfmt::skip]
    #[test]
    fn test_lock() {
// ANCHOR: simple_test
let book = Book::builder()
    .title("The Catcher in the Rye")
    .author("J.D. Salinger")
    .year(1951)
    .build()
    .expect("Failed to create Book object");

// Neither set nor laziliy initialized.
assert!(!book.has_location());
book.write_location().store("R42.S1".to_string());
assert_eq!(*book.location(), "R42.S1");
// ANCHOR_END: simple_test
    }
}

fn main() {}
