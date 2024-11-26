use super::FromNestAttr;
use crate::{FXSynValue, FXTriggerHelper};
use darling::{util::Flag, FromMeta};

#[derive(Debug, Clone, FromMeta)]
pub struct FXFallible<T = FXSynValue<syn::Path>>
where
    T: FromMeta,
{
    off:        Flag,
    #[darling(rename = "error")]
    error_type: Option<T>,
    // error_type: Option<FXSynValue<syn::Path>>,
}

impl<T> FXFallible<T>
where
    T: FromMeta,
{
    pub fn error_type(&self) -> Option<&T> {
        self.error_type.as_ref()
    }
}

impl<T> FXTriggerHelper for FXFallible<T>
where
    T: FromMeta,
{
    fn is_true(&self) -> bool {
        !self.off.is_present()
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
