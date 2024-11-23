pub use crate::errors::FieldXError;

pub trait FXStruct {}

#[doc(hidden)]
pub trait FXNewDefault {
    type Builder;
    type Value;

    fn new_default(builder: Self::Builder, value: Option<Self::Value>) -> Self;
}
