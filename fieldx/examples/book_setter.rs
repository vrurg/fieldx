#[rustfmt::skip]
#[allow(unused)]
mod setter {
//ANCHOR: set_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
    author: String,
    year:   u32,
    #[fieldx(get(copy), set)]
    available: u16,
    #[fieldx(set("place_into"))]
    location: String,
}
// ANCHOR_END: set_decl

#[test]
fn test_book_accessor() {
// ANCHOR: set_usage
let mut book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .available(1)
    .location("R12.S2".to_string()) // Row 12, Section 2
    .build()
    .expect("Failed to create book");

book.set_available(2);
// ANCHOR: set_location
book.place_into("R12.S3".to_string());
// ANCHOR_END: set_location
assert_eq!(book.available(), 2);
assert_eq!(book.location(), "R12.S3");
// ANCHOR_END: set_usage
}
}

fn main() {}
