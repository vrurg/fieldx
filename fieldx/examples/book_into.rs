#[rustfmt::skip]
#[allow(unused)]
mod setter {
//ANCHOR: into_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
    author: String,
    year:   u32,
    #[fieldx(get(copy), set)]
    available: u16,
    #[fieldx(set("place_into", into))]
    location: String,
}
// ANCHOR_END: into_decl

#[test]
fn test_book_accessor() {
// ANCHOR: into_usage
let mut book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .available(1)
    .location("R12.S2".to_string()) // Row 12, Section 2
    .build()
    .expect("Failed to create book");

book.place_into("R12.S3");
assert_eq!(book.location(), "R12.S3");
// ANCHOR_END: into_usage
}
}

fn main() {}
