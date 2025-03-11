//! The most basic helper declaration

use crate::{
    set_literals, FXAttributes, FXBool, FXOrig, FXProp, FXSetState, FXString, FXTriggerHelper, FXTryInto, FromNestAttr,
};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

/// Minimal helper declaration. For example, `fieldx` uses it for helpers like `reader` or `writer`.
///
/// The `BOOL_ONLY` parameter disables the literal subargument that specifies custom helper name. For example, with
/// the following declaraion, argument `foo("my_name")` results in an error:
///
/// ```ignore
///     foo: FXNestingAttr<FXBaseHelper<true>>,
/// ```
#[fxhelper]
#[derive(Default, Debug)]
pub struct FXBaseHelper<const BOOL_ONLY: bool = false> {}

impl<const BOOL_ONLY: bool> FXBaseHelper<BOOL_ONLY> {
    fn allowed_literals(&self, literals: &Vec<Lit>) -> darling::Result<()> {
        if BOOL_ONLY {
            return Err(self.no_literals(literals).unwrap_err());
        }
        Ok(())
    }
}

impl<const BOOL_ONLY: bool> FromNestAttr for FXBaseHelper<BOOL_ONLY> {
    set_literals! {helper, ..1usize => name as Lit::Str; pre_validate => allowed_literals}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl<const BOOL_ONLY: bool> From<FXBaseHelper<BOOL_ONLY>> for bool {
    fn from(value: FXBaseHelper<BOOL_ONLY>) -> Self {
        *value.is_true()
    }
}

impl<const BOOL_ONLY: bool> From<&FXBaseHelper<BOOL_ONLY>> for bool {
    fn from(value: &FXBaseHelper<BOOL_ONLY>) -> Self {
        *value.is_true()
    }
}

impl<const BOOL_ONLY: bool> From<bool> for FXBaseHelper<BOOL_ONLY> {
    fn from(value: bool) -> Self {
        Self {
            off: Flag::from(value),
            ..Default::default()
        }
    }
}
