pub use crate::errors::FieldXError;

pub trait FXStruct {}

#[doc(hidden)]
pub trait FXNewDefault<O, T> {
    fn new_default(builder_method: fn(&O) -> T, value: Option<T>) -> Self;
}
