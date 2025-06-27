#![allow(unused)]

#[rustfmt::skip]
mod visibility {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: visibility_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
pub struct Book {
    title:  String,
    author: String,
    year:   u32,
    #[fieldx(
        get(copy, vis(pub(crate))),
        get_mut(private),
        inner_mut,
        builder(private)
    )]
    available: u16,
    #[fieldx(
        set("place_into", private),
        inner_mut,
        default("unknown".to_string()),
        builder(private)
    )]
    location: String,
}
// ANCHOR_END: visibility_decl

#[test]
fn test_book_accessor() {
// ANCHOR: visibility_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .available(42)
    .build()
    .expect("Failed to create book");

// ANCHOR_END: visibility_usage
}
}

fn main() {}
