pub use crate::error::FieldXError;
use std::fmt::Debug;

/// Marks struct as implementing `fieldx` functionality.
pub trait FXStruct {}

#[doc(hidden)]
pub trait FXBuilderWrapper {
    type Owner: FXStruct;
    type Value;
    type Error: Debug;
}
