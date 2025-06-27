#![allow(unused)]

#[rustfmt::skip]
mod referenced {
//ANCHOR: ref_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
    author: String,
    year:   u32,
}
// ANCHOR_END: ref_decl

#[test]
fn test_book_accessor() {
// ANCHOR: ref_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");

let title: &str = book.title();
let author: &str = book.author();
let year: &u32 = book.year();
assert_eq!(title, "The Hitchhiker's Guide to the Galaxy");
assert_eq!(author, "Douglas Adams");
assert_eq!(year, &1979);
// ANCHOR_END: ref_usage
}
}

#[rustfmt::skip]
mod copied {
//ANCHOR: copy_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
    #[fieldx(get(clone))]
    author: String,
    #[fieldx(get(copy))]
    year:   u32,
}
// ANCHOR_END: copy_decl

#[test]
fn test_book_accessor() {
// ANCHOR: copy_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");

let title: &str = book.title();
let author: String = book.author();
let year: u32 = book.year();
assert_eq!(title, "The Hitchhiker's Guide to the Galaxy");
assert_eq!(author, "Douglas Adams".to_string());
assert_eq!(year, 1979);
// ANCHOR_END: copy_usage
}
}

#[rustfmt::skip]
#[allow(unused)]
mod mutable {
//ANCHOR: mut_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
    author: String,
    #[fieldx(get(copy))]
    year:   u32,
    #[fieldx(get(copy), get_mut, builder(off), default(false))]
    borrowed: bool,
}
// ANCHOR_END: mut_decl

#[test]
fn test_book_accessor() {
// ANCHOR: mut_usage
let mut book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");

assert!(!book.borrowed());
*book.borrowed_mut() = true;
assert!(book.borrowed());
// ANCHOR_END: mut_usage
}
}

#[rustfmt::skip]
#[allow(unused)]
mod rename {
//ANCHOR: rename_decl
use fieldx::fxstruct;

#[fxstruct(get("get_"), builder)]
struct Book {
    title:  String,
    author: String,
    #[fieldx(get("published", copy))]
    year:   u32,
}
// ANCHOR_END: rename_decl

#[test]
fn test_book_accessor() {
// ANCHOR: rename_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");

assert_eq!(book.get_title(), "The Hitchhiker's Guide to the Galaxy");
assert_eq!(book.get_author(), "Douglas Adams");
assert_eq!(book.published(), 1979);
// ANCHOR_END: rename_usage
}
}

fn main() {}
