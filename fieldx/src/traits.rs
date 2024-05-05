pub use crate::errors::FieldXError;

pub trait FXStruct {}

pub trait FXStructNonSync: FXStruct {
}

pub trait FXStructSync: FXStruct {
    fn __fieldx_init(&self);
}
