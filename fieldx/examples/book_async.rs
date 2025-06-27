#![allow(unused)]
#[cfg(feature = "async")]
#[rustfmt::skip]
mod simple {
    use fieldx::fxstruct;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Instant;

    // ANCHOR: simple_decl
#[fxstruct(get)]
struct RegistryRecord {
    location: String,
    #[fieldx(mode(async), lock, get(copy), get_mut)]
    available: u32,
    #[fieldx(optional, get(as_ref))]
    signed_by: String,
}

#[fxstruct(get, builder(into))]
struct Book {
    title:     String,
    author:    String,
    #[fieldx(get(copy), builder(into(off)))]
    year:      u32,
    #[fieldx(get(copy), builder(into(off)))]
    bar_code:  u32,
    #[fieldx(r#async, lazy)]
    registry_record: RegistryRecord,
}

impl Book {
    async fn build_registry_record(&self) -> RegistryRecord {
        self.request_registry_record(self.bar_code).await
    }
}
    // ANCHOR_END: simple_decl

impl Book {
    async fn request_registry_record(&self, _bar_code: u32) -> RegistryRecord {
        RegistryRecord {
            location: String::from("R42.S1"), // Row 42, Section 1
            available: 1.into(),
            signed_by: Some("S.K.".to_string()),
        }
    }
}

    #[tokio::test]
    async fn test_lock() {
// ANCHOR: simple_usage
let book = Book::builder()
    .title("The Catcher in the Rye")
    .author("J.D. Salinger")
    .year(1951)
    .bar_code(123456)
    .build()
    .expect("Failed to create Book object");

let registry_record = book.registry_record().await;
*registry_record.available_mut().await -= 1;
// ANCHOR_END: simple_usage
assert_eq!(*registry_record.location(), "R42.S1");
assert_eq!(registry_record.available().await, 0);
assert_eq!(registry_record.signed_by(), Some(&"S.K.".to_string()));
    }
}

fn main() {}
