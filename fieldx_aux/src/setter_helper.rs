use crate::set_literals;
use crate::FXAttributes;
use crate::FXBool;
use crate::FXOrig;
use crate::FXProp;
use crate::FXPropBool;
use crate::FXSetState;
use crate::FXString;
use crate::FXTryInto;
use crate::FromNestAttr;

use darling::util::Flag;
use darling::FromMeta;
use fieldx_derive_support::fxhelper;
use getset::Getters;
use proc_macro2::TokenStream;
use syn::Lit;

#[fxhelper(to_tokens)]
#[derive(Default, Debug)]
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
    set_literals! { setter, ..1 => name; pre_validate => allowed_literals }

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}
