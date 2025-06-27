#![allow(unused)]

#[rustfmt::skip]
mod no_builder {
// ANCHOR: declaration
use fieldx::fxstruct;
#[fxstruct]
struct Book {
    #[fieldx(get, set)]
    title: String,
}
// ANCHOR_END: declaration

#[test]
#[rustfmt::skip]
fn test_my_struct() {
// ANCHOR: usage
let mut my_struct = Book::new();
my_struct.set_title("The Hitchhiker's Guide To The Galaxy".to_string());
assert_eq!(my_struct.title(), "The Hitchhiker's Guide To The Galaxy");
// ANCHOR_END: usage
}
}

#[rustfmt::skip]
mod with_builder {
// ANCHOR: builder_decl
use fieldx::fxstruct;
#[fxstruct(builder, get)]
struct Book {
    title: String,
    author: String,
    #[fieldx(get(copy))]
    year: u32,
    // How many books are available in the depository.
    #[fieldx(get(copy), builder(off))]
    available: u32,
}
// ANCHOR_END: builder_decl

#[test]
#[rustfmt::skip]
fn test_my_struct() {
// ANCHOR: builder_usage
let my_struct = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to build Book object");

assert_eq!(my_struct.title(), "The Hitchhiker's Guide to the Galaxy");
assert_eq!(my_struct.author(), "Douglas Adams");
assert_eq!(my_struct.year(), 1979);
assert_eq!(my_struct.available(), 0);
// ANCHOR_END: builder_usage
}

}

fn main() {}
