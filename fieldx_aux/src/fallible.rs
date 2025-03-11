//! Argument that signals possibility of errors.
use super::FromNestAttr;
use crate::{FXProp, FXPropBool, FXSetState, FXSynValue, FXTriggerHelper};
use darling::{util::Flag, FromMeta};

/// This argument can be used to mark, say, methods as returning a `Result` and specify what error type is expected.
#[derive(Debug, Clone, FromMeta)]
pub struct FXFallible<T = FXSynValue<syn::Path>>
where
    T: FromMeta,
{
    off:        Flag,
    #[darling(rename = "error")]
    error_type: Option<T>,
}

impl<T> FXFallible<T>
where
    T: FromMeta,
{
    /// Accessor for the error type.
    pub fn error_type(&self) -> Option<&T> {
        self.error_type.as_ref()
    }
}

impl<T> FXTriggerHelper for FXFallible<T>
where
    T: FromMeta,
{
    fn is_true(&self) -> FXProp<bool> {
        FXProp::from(self.off).not()
    }
}

impl<T> FXSetState for FXFallible<T>
where
    T: FromMeta,
{
    fn is_set(&self) -> FXProp<bool> {
        FXProp::from(self.off).not()
    }
}

impl<T> FromNestAttr for FXFallible<T>
where
    T: FromMeta,
{
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self {
            off:        Flag::default(),
            error_type: None,
        })
    }
}
