pub use crate::errors::UninitializedFieldError;
use std::sync::Arc;

pub trait FXStruct: Sized {}

pub trait FXStructNonSync: FXStruct {
    fn __fieldx_new() -> Self;
}

pub trait FXStructSync: FXStruct {
    fn __fieldx_init(self) -> Arc<Self>;
    fn __fieldx_new() -> Arc<Self>;
}

pub trait FXStructBuilder<T>
where
    T: FXStruct,
{
    fn build(&mut self) -> std::result::Result<T, UninitializedFieldError>;
}

pub trait FXStructBuilderSync<T>
where
    T: FXStruct,
{
    fn build(&mut self) -> std::result::Result<Arc<T>, UninitializedFieldError>;
}
