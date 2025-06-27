#![allow(unused)]

#[rustfmt::skip]
mod opt_in {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: opt_in_decl
use fieldx::fxstruct;

#[fxstruct(get, builder(opt_in))]
struct Book {
    #[fieldx(builder)]
    title:  String,
    #[fieldx(builder)]
    author: String,
    #[fieldx(builder)]
    year:   u32,
    #[fieldx(get(copy), get_mut, inner_mut)]
    available: u16,
    #[fieldx(
        set("place_into"),
        inner_mut,
        default("unknown".to_string())
    )]
    location: String,
}
// ANCHOR_END: opt_in_decl

#[test]
fn test_book_accessor() {
// ANCHOR: opt_in_usage
let book = BookBuilder::new()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");
// ANCHOR_END: opt_in_usage
}
}

#[rustfmt::skip]
mod opt_in_with_subargs {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: opt_in_subarg_decl
use fieldx::fxstruct;

#[fxstruct(get, builder("BookConstructor", opt_in, prefix("set_")))]
struct Book {
    #[fieldx(builder)]
    title:  String,
    #[fieldx(builder)]
    author: String,
    #[fieldx(builder)]
    year:   u32,
    #[fieldx(get(copy), get_mut, inner_mut)]
    available: u16,
    #[fieldx(
        set("place_into"),
        inner_mut,
        default("unknown".to_string())
    )]
    location: String,
}
// ANCHOR_END: opt_in_subarg_decl

#[test]
fn test_book_accessor() {
// ANCHOR: opt_in_subarg_usage
let book = BookConstructor::new()
    .set_title("The Hitchhiker's Guide to the Galaxy".to_string())
    .set_author("Douglas Adams".to_string())
    .set_year(1979)
    .build()
    .expect("Failed to create book");
// ANCHOR_END: opt_in_subarg_usage
}
}

#[rustfmt::skip]
mod with_default {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: with_default_decl
use fieldx::fxstruct;

#[fxstruct(get)]
struct Book {
    #[fieldx(builder)]
    title:  String,
    #[fieldx(builder)]
    author: String,
    #[fieldx(builder)]
    year:   u32,
    #[fieldx(get(copy), get_mut, inner_mut)]
    available: u16,
    #[fieldx(
        set("place_into"),
        inner_mut,
        default("unknown".to_string()),
        builder
    )]
    location: String,
}
// ANCHOR_END: with_default_decl

#[test]
fn test_book_accessor() {
// ANCHOR: with_default_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");

assert_eq!(*book.location(), "unknown");
// ANCHOR_END: with_default_usage
}
}

#[rustfmt::skip]
mod optional {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: opt_decl
use fieldx::fxstruct;

#[fxstruct(get, builder)]
struct Book {
    title:  String,
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
// ANCHOR_END: opt_decl

#[test]
fn test_book_accessor() {
// ANCHOR: opt_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy".to_string())
    .author("Douglas Adams".to_string())
    .year(1979)
    .build()
    .expect("Failed to create book");

assert!(book.signed_by().is_none());
// ANCHOR_END: opt_usage
}
}

#[rustfmt::skip]
mod coercion {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    New, Mint, Used
}

//ANCHOR: coerce_decl
use fieldx::fxstruct;

#[fxstruct(get(clone), builder(into))]
struct Book {
    title:  String,
    author: String,
    #[fieldx(get(copy), builder(into(off)))]
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
// ANCHOR_END: coerce_decl

#[test]
fn test_book_accessor() {
// ANCHOR: coerce_usage
let book = Book::builder()
    .title("The Hitchhiker's Guide to the Galaxy")
    .author("Douglas Adams")
    .year(1979)
    .signed_by("Douglas Adams")
    .build()
    .expect("Failed to create book");

assert_eq!(book.title(), "The Hitchhiker's Guide to the Galaxy".to_string());
assert_eq!(book.author(), "Douglas Adams".to_string());
assert_eq!(book.year(), 1979);
assert_eq!(book.signed_by(), Some("Douglas Adams".to_string()));
// ANCHOR_END: coerce_usage
}
}

fn main() {}
