#![doc(html_root_url = "https://docs.rs/fieldx_core/0.2.0/")]
//! This crates implements basic elements of the core functionality of the FieldX. It can be used by 3rd-party crates
//! to extend the FieldX functionality or implement their own proc-macros.
//!
//! **Note:** Unfortunately, the lack of time doesn't allow to properly document this crate. But the time will come for it!
pub mod codegen;
pub mod ctx;
pub mod field_receiver;
pub mod struct_receiver;
pub mod types;
pub mod util;
