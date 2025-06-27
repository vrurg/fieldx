#[cfg(feature = "sync")]
#[allow(unused)]
#[rustfmt::skip]
mod std {
#[fxstruct(get)]
struct Book {
    title:    String,
    author:   String,
    year:     u32,
    isbn:     String,
    #[fieldx(set("place_into", into))]
    location: String,
}

use std::collections::HashMap;
use std::sync::Arc;

// ANCHOR: rc_decl
use fieldx::fxstruct;

#[fxstruct(sync, rc, builder)]
struct LibraryInventory {
    /// Map ISBN to Book
    #[fieldx(inner_mut, get, get_mut)]
    books: HashMap<String, Book>,
}

impl LibraryInventory {
    fn make_an_inventory(&self) {
        // The inventory must be available at all times for being able to serve readers. The `books` field is
        // lock-protected. So, we take care as of not locking it for too long by utilizing this naive approach.
        let isbns = self.books().keys().cloned().collect::<Vec<_>>();

        for isbn in isbns {
            if let Some(book) = self.books_mut().get_mut(&isbn) {
                // Check whatever needs to be checked about the book
                self.check_book(book);
            }
        }
    }

    fn open_the_day(&self) {
        let myself: Arc<LibraryInventory> = self.myself().unwrap();

        let inv_task = std::thread::spawn(move || {
            myself.make_an_inventory();
        });

        self.respond_to_readers();

        inv_task.join().expect("Inventory task failed");
    }
}
// ANCHOR_END: rc_decl

impl LibraryInventory {
    fn respond_to_readers(&self) {
        // Respond to readers' requests
        // This method can be called concurrently with `make_an_inventory`
        // because `books` is protected by a lock.
    }

    fn check_book(&self, _book: &mut Book) {
        // Check the book's condition, availability, etc.
        // This method can also be called concurrently with `respond_to_readers`
        // because `books` is protected by a lock.
    }
}
}

fn main() {
    // let inv = LibraryInventory::builder()
    //     .books(HashMap::new())
    //     .build()
    //     .expect("Failed to create LibraryInventory");

    // inv.open_the_day();
}
