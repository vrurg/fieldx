#![doc(html_root_url = "https://docs.rs/fieldx/0.2.0/")]
//! # FieldX
//!
//! `fieldx` is a declarative object orchestrator that streamlines object and dependency management. It supports:
//!
//! - Lazy initialization of fields with builder methods that simplifies implicit dependency management
//! - Accessor and setter methods for fields
//! - Optional field infrastructure
//! - Sync-safe field management with locks
//! - Struct builder pattern
//! - Post-build hook for validation and adjustment of struct
//! - `serde` support
//! - Type conversions using `Into` trait
//! - Default values for fields
//! - Inner mutability for fields
//! - Pass-through attributes for fields, methods, and generated helper structs
//! - Renaming for generated methods names and serialization inputs/outputs
//! - Generic structs
//! - Visibility control for generated methods and helper structs
//!
//! ## Quick Start
//!
//! Let's start with an example:
//!
//! ```
//! # use std::cell::RefCell;
//! use fieldx::fxstruct;
//!
//! #[fxstruct(lazy)]
//! struct Foo {
//!     count: usize,
//!     foo:   String,
//!     // This declaration can be replaced with:
//!     //     #[fieldx(lazy(off), inner_mut, get, get_mut)]
//!     //     order: Vec<&'static str>,
//!     // But we want things here be a bit more explicit for now.
//!     #[fieldx(lazy(off), get)]
//!     order: RefCell<Vec<&'static str>>,
//! }
//!
//! impl Foo {
//!     fn build_count(&self) -> usize {
//!         self.order.borrow_mut().push("Building count.");
//!         12
//!     }
//!
//!     fn build_foo(&self) -> String {
//!         self.order.borrow_mut().push("Building foo.");
//!         format!("foo is using count: {}", self.count())
//!     }
//! }
//!
//! # fn main() {
//! let foo = Foo::new();
//! assert_eq!(foo.order().borrow().len(), 0);
//! assert_eq!(foo.foo(), "foo is using count: 12");
//! assert_eq!(foo.foo(), "foo is using count: 12");
//! assert_eq!(foo.order().borrow().len(), 2);
//! assert_eq!(foo.order().borrow()[0], "Building foo.");
//! assert_eq!(foo.order().borrow()[1], "Building count.");
//! # }
//! ```
//!
//! What happens here is:
//!
//! - A struct where all fields are `lazy` by default, meaning they are lazily initialized using corresponding
//!   `build_<field_name>` methods that provide the initial values.
//! - Laziness is explicitly disabled for the `order` field, meaning it will be initialized with its default value.
//!
//! At run-time, we first ensure that the `order` vector is empty, i.e., none of the `build_` methods was called. Then
//! we read from `foo` using its accessor method, resulting in the field's builder method being called. The method, in turn,
//! uses the `count` field via its accessor method, which also invokes `count`'s builder method.
//!
//! Each builder method updates the `order` field with a message indicating that it was called. Then we make sure that
//! each `build_` method was invoked only once.
//!
//! It must be noticeable that a minimal amount of handcraft is needed here as most of the boilerplate is handled by the `fxstruct` attribute,
//! which even provides a basic `new()` constructor for the struct.
//!
//! Further information is provided in the [FieldX Object Manager](https://vrurg.github.io/fieldx/) book.
//!
//! ## Feature Flags
//!
//! The following feature flags are supported by this crate:
//!
//! | *Feature* | *Description* |
//! |-|-|
//! | **sync** | Support for sync-safe mode of operation |
//! | **async** | Support for async mode of operation |
//! | **tokio-backend** | Selects the Tokio backend for async mode. A no-op without the `async` feature. |
//! | **async-lock-backend** | Selects the `async-lock` backend for async mode. A no-op without the `async` feature. |
//! | **async-tokio** | Combines `async` and `tokio-backend` features. |
//! | **async-lock** | Combines `async` and `async-lock-backend` features. |
//! | **clonable-lock** | Enables the [clonable lock wrapper type](more_on_locks.md). |
//! | **send_guard** | See corresponding feature of the [`parking_lot` crate](https://crates.io/crates/parking_lot) |
//! | **serde** | Enable support for `serde` marshalling. |
//! | **diagnostics** | Enable additional diagnostics for compile time errors. Experimental, requires Rust nightly toolset. |
//!
//! **Note:** The `tokio-backend` and `async-lock-backend` features are mutually exclusive. You can only use one of them
//! at a time or FieldX will produce a compile-time error.

#[cfg(feature = "async")]
pub mod r#async;
pub mod error;
pub mod lock_guards;
pub mod plain;
#[cfg(feature = "sync")]
pub mod sync;
pub mod traits;

pub use fieldx_aux;
#[doc(hidden)]
pub use fieldx_aux::FXOrig;
pub use fieldx_core;
#[doc(inline)]
pub use fieldx_derive::fxstruct;
#[cfg(feature = "async")]
#[doc(hidden)]
pub use std::fmt;
#[doc(hidden)]
pub use std::sync::atomic::Ordering;

#[cfg(feature = "async")]
#[doc(hidden)]
#[allow(unused_imports)]
use r#async as doc_async;
