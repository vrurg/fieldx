use crate::{
    set_literals, FXAttributes, FXBool, FXOrig, FXProp, FXSetState, FXString, FXTriggerHelper, FXTryInto, FromNestAttr,
};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

#[fxhelper]
#[derive(Default, Debug, Getters)]
pub struct FXSetterHelper<const BOOL_ONLY: bool = false> {
    #[getset(get = "pub")]
    into: Option<FXBool>,
}

impl<const BOOL_ONLY: bool> FXSetterHelper<BOOL_ONLY> {
    fn allowed_literals(&self, literals: &Vec<Lit>) -> darling::Result<()> {
        if BOOL_ONLY {
            return Err(self.no_literals(literals).unwrap_err());
        }
        Ok(())
    }

    #[inline]
    pub fn is_into(&self) -> Option<FXProp<bool>> {
        self.into.as_ref().map(|into| into.into())
    }
}

impl<const BOOL_ONLY: bool> FromNestAttr for FXSetterHelper<BOOL_ONLY> {
    set_literals! { setter, ..1 => name as Lit::Str; pre_validate => allowed_literals }

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}
