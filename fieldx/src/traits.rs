pub use crate::error::FieldXError;
use std::fmt::Debug;

pub trait FXStruct {}

pub trait FXBuilderWrapper {
    type Owner: FXStruct;
    type Value;
    type Error: Debug;
}
