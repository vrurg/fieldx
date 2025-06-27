#[cfg(feature = "serde")]
#[allow(unused)]
mod two_way {
    // ANCHOR: ser_decl
    use fieldx::fxstruct;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Clone)]
    #[fxstruct(get, builder, serde)]
    pub struct Book {
        title:     String,
        author:    String,
        #[fieldx(get(copy))]
        year:      u32,
        #[fieldx(optional, get(as_ref))]
        signed_by: String,
        #[fieldx(set, inner_mut, default("unknown".to_string()))]
        location:  String,
    }
    // ANCHOR_END: ser_decl

    pub fn serialize_book() -> String {
        // ANCHOR: ser_usage
        let book = Book::builder()
            .title("The Hitchhiker's Guide to the Galaxy".to_string())
            .author("Douglas Adams".to_string())
            .year(1979)
            .signed_by("Douglas Adams".to_string())
            .location("Shelf 42".to_string())
            .build()
            .expect("Failed to create book");

        serde_json::to_string_pretty(&book).expect("Failed to serialize book")
        // ANCHOR_END: ser_usage
    }

    #[cfg(test)]
    mod tests {
        #[test]
        #[rustfmt::skip]
        pub fn test_serialize_book() {
// ANCHOR: ser_test
let serialized = r#"{
  "title": "The Hitchhiker's Guide to the Galaxy",
  "author": "Douglas Adams",
  "year": 1979,
  "signed_by": "Douglas Adams",
  "location": "Shelf 42"
}"#;

let deserialized: super::Book = serde_json::from_str(&serialized).expect("Failed to deserialize book");
assert_eq!(*deserialized.title(), "The Hitchhiker's Guide to the Galaxy");
assert_eq!(*deserialized.author(), "Douglas Adams");
assert_eq!(deserialized.year(), 1979);
assert_eq!(deserialized.signed_by(), Some(&String::from("Douglas Adams")));
assert_eq!(*deserialized.location(), "Shelf 42");
// ANCHOR_END: ser_test
        }
    }
}

#[cfg(feature = "serde")]
    #[rustfmt::skip]
mod defaults {
    use fieldx::fxstruct;
    use serde::Deserialize;
    use serde::Serialize;

// ANCHOR: defaults_decl
#[derive(Clone)]
#[fxstruct(get, serde)]
pub struct Foo {
    #[fieldx(
        default("constructor".to_string()),
        serde(
            default("deserialization".to_string())
        )
    )]
    source: String,
}
// ANCHOR_END: defaults_decl

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_foo_defaults() {
// ANCHOR: defaults_test
let foo = Foo::new();
assert_eq!(*foo.source(), "constructor");

let deserialized = serde_json::from_str::<Foo>("{}").expect("Failed to deserialize Foo");
assert_eq!(*deserialized.source(), "deserialization");
// ANCHOR_END: defaults_test
        }
    }
}

fn main() {
    #[cfg(feature = "serde")]
    {
        println!("{}", two_way::serialize_book());
    }
}
