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

pub trait FXStructBuilder
{
    type TargetStruct: FXStructNonSync;
    fn build(&mut self) -> std::result::Result<Self::TargetStruct, UninitializedFieldError>;
}

pub trait FXStructBuilderSync
{
    type TargetStruct: FXStructSync;
    fn build(&mut self) -> std::result::Result<Arc<Self::TargetStruct>, UninitializedFieldError>;
}
