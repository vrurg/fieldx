#![allow(unused)]

#[rustfmt::skip]
mod setter {
//ANCHOR: imut_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
    author: String,
    year:   u32,
    #[fieldx(get(copy), get_mut, inner_mut)]
    available: u16,
    #[fieldx(set("place_into"), inner_mut)]
    location: String,
}
// ANCHOR_END: imut_decl

#[test]
fn test_book_accessor() {
// ANCHOR: imut_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .available(1)
    .location("R12.S2".to_string()) // Row 12, Section 2
    .build()
    .expect("Failed to create book");

*book.available_mut() = 3;
book.place_into("R12.S4".to_string());
assert_eq!(book.available(), 3);
assert_eq!(*book.location(), "R12.S4");
// ANCHOR_END: imut_usage
}
}

fn main() {}
