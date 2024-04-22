use super::{FXAttributes, FXHelperTrait, FromNestAttr};
use crate::util::set_literals;
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

#[fxhelper]
#[derive(FromMeta, Default, Debug, Clone)]
pub(crate) struct FXBaseHelper<const BOOL_ONLY: bool = false> { }

impl<const BOOL_ONLY: bool> FXBaseHelper<BOOL_ONLY> {
    fn allowed_literals(&self, literals: &Vec<Lit>) -> darling::Result<()> {
        if !BOOL_ONLY {
            return Ok(());
        }
        self.no_literals(literals)
    }
}

// impl<const BOOL_ONLY: bool> FXHelperTrait for FXBaseHelper<BOOL_ONLY> {
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

impl<const BOOL_ONLY: bool> FromNestAttr for FXBaseHelper<BOOL_ONLY> {
    set_literals! {helper, ..1usize => rename as Lit::Str; pre_validate => allowed_literals}

    fn for_keyword() -> Self {
        Self::default()
    }
}

impl<const BOOL_ONLY: bool> From<FXBaseHelper<BOOL_ONLY>> for bool {
    fn from(value: FXBaseHelper<BOOL_ONLY>) -> Self {
        value.is_true()
    }
}

impl<const BOOL_ONLY: bool> From<&FXBaseHelper<BOOL_ONLY>> for bool {
    fn from(value: &FXBaseHelper<BOOL_ONLY>) -> Self {
        value.is_true()
    }
}

impl<const BOOL_ONLY: bool> From<bool> for FXBaseHelper<BOOL_ONLY> {
    fn from(value: bool) -> Self {
        Self {
            off:           Flag::from(value),
            .. Default::default()
        }
    }
}
