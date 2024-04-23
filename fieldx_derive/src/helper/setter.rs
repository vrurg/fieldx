use super::FromNestAttr;
use crate::util::set_literals;
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

#[fxhelper]
#[derive(Default, Debug, Getters)]
pub(crate) struct FXSetterHelper<const BOOL_ONLY: bool = false> {
    #[getset(get = "pub(crate)")]
    into: Option<bool>,
}

impl<const BOOL_ONLY: bool> FXSetterHelper<BOOL_ONLY> {
    fn allowed_literals(&self, literals: &Vec<Lit>) -> darling::Result<()> {
        if !BOOL_ONLY {
            return Ok(());
        }
        self.no_literals(literals)
    }

    #[inline]
    pub(crate) fn is_into(&self) -> Option<bool> {
        self.into
    }
}

// impl<const BOOL_ONLY: bool> FXHelperTrait for FXSetterHelper<BOOL_ONLY> {
//     fn is_true(&self) -> bool {
//         !self.off.is_present()
//     }

//     fn rename(&self) -> Option<&str> {
//         self.rename.as_deref()
//     }

//     fn attributes_fn(&self) -> Option<&FXAttributes> {
//         self.attributes_fn.as_ref()
//     }
// }

impl<const BOOL_ONLY: bool> FromNestAttr for FXSetterHelper<BOOL_ONLY> {
    set_literals! { setter, ..1 => rename as Lit::Str; pre_validate => allowed_literals }

    fn for_keyword() -> Self {
        Self::default()
    }
}
