#![allow(unused)]

#[rustfmt::skip]
mod clonable {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: attr_decl
use fieldx::fxstruct;

#[fxstruct(
    get,
    builder(
        attributes(
            derive(Clone, PartialEq, Eq, Debug)
        ),
    )
)]
struct Book {
    #[fieldx(into)]
    title:  String,
    #[fieldx(into)]
    author: String,
    year:   u32,
    #[fieldx(optional)]
    signed_by: String,
    #[fieldx(
        set("place_into"),
        inner_mut,
        default("unknown".to_string())
    )]
    location: String,
}
// ANCHOR_END: attr_decl

#[test]
fn test_book_accessor() {
// ANCHOR: attr_usage
let book_builder = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy")
    .author("Douglas Adams")
    .year(1979);

let book_builder_clone = book_builder.clone();

assert_eq!(book_builder_clone, book_builder);
// ANCHOR_END: attr_usage
}
}

fn main() {}
