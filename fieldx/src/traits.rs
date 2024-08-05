pub use crate::errors::FieldXError;

pub trait FXStruct {}

pub trait FXStructNonSync: FXStruct {}

pub trait FXStructSync: FXStruct {}

#[doc(hidden)]
pub trait FXNewDefault<O, T> {
    fn new_default(builder_method: fn(&O) -> T, value: Option<T>) -> Self;
}
