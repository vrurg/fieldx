use super::FromNestAttr;
use crate::{FXSynValue, FXTriggerHelper};
use darling::{util::Flag, FromMeta};

#[derive(Debug, Clone, FromMeta)]
pub struct FXFallible {
    off:        Flag,
    #[darling(rename = "error")]
    error_type: Option<FXSynValue<syn::Path>>,
}

impl FXFallible {
    pub fn error_type(&self) -> Option<&FXSynValue<syn::Path>> {
        self.error_type.as_ref()
    }
}

impl FXTriggerHelper for FXFallible {
    fn is_true(&self) -> bool {
        !self.off.is_present()
    }
}

impl FromNestAttr for FXFallible {
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self {
            off:        Flag::default(),
            error_type: None,
        })
    }
}
