#![allow(unused)]

#[doc(hidden)]
use fieldx::fxstruct;

// ANCHOR: doc_decl
/// The struct.
#[fxstruct(get, builder(doc("This is our builder for the Book struct.")))]
pub struct Book {
    title:     String,
    author:    String,
    year:      u32,
    #[fieldx(optional)]
    signed_by: String,
    #[fieldx(
        set(
            doc(
                "Set the physical location of the book.",
                "",
                "I have no idea what else to add to the above.\n\nBut I just need another line here!",
            )
        ),
        builder(doc("Initial book location.")),
        inner_mut,
        default("unknown".to_string())
    )]
    location:  String,
}
// ANCHOR_END: doc_decl

fn main() {}
