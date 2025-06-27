#![allow(unused)]
#[cfg(feature = "sync")]
mod simple {
    use fieldx::fxstruct;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Instant;

    // ANCHOR: simple_decl
    #[rustfmt::skip]
#[fxstruct(get, builder(into))]
struct Book {
    title:     String,
    author:    String,
    #[fieldx(get(copy), builder(into(off)))]
    year:      u32,
    #[fieldx(optional)]
    signed_by: String,
    location:  String,
    #[fieldx(reader, writer, get(copy), get_mut, builder(into(off)))]
    available: u32,
    // Map borrower IDs to the time they borrowed the book.
    #[fieldx(lock, get_mut, builder(off))]
    borrowers: HashMap<String, Instant>,
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
    .signed_by("S.K.")
    .location("R42.S1".to_string()) // Row 12, Section 2
    .available(50)
    .build()
    .expect("Failed to create Book object");

let borrowers = 30;
let barrier = std::sync::Barrier::new(borrowers + 1);

thread::scope(|s| {
    for i in 0..borrowers {
        let book_ref = &book;
        let idx = i;
        let barrier = &barrier;
        s.spawn(move || {
            // Try to ensure real concurrency by making all threads start at the same time.
            barrier.wait();
            *book_ref.write_available() -= 1;
            book_ref.borrowers_mut().insert(format!("user{idx}"), Instant::now());
        });
    }

    barrier.wait();
});

assert_eq!(book.available(), 20);
assert_eq!(*book.read_available(), 20);
assert_eq!(book.borrowers().len(), borrowers as usize);
// ANCHOR_END: simple_test
    }
}

fn main() {}
