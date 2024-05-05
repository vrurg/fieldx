pub use crate::errors::FieldXError;

pub trait FXStruct {}

pub trait FXStructNonSync: FXStruct {}

pub trait FXStructSync: FXStruct {}
