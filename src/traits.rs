pub use crate::errors::UninitializedFieldError;
use std::sync::Arc;

pub trait FXStruct: Sized {}

pub trait FXStructBuilder<T>
where
    T: FXStruct,
{
    fn build(&mut self) -> std::result::Result<T, UninitializedFieldError>;
}

pub trait FXStructBuilderSync<T>
where
    T: FXStruct + Sync + Send,
{
    fn build(&mut self) -> std::result::Result<Arc<T>, UninitializedFieldError>;
}
