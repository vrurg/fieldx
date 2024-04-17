use super::{FXHelperSwitch, FXHelperTrait, FromNestAttr};
use crate::util::set_literals;
use darling::FromMeta;
use getset::Getters;
use syn::Lit;

#[derive(FromMeta, Default, Debug, Clone, Getters)]
pub(crate) struct FXBaseHelper<const BOOL_ONLY: bool = false> {
    #[getset(skip)]
    rename: Option<String>,
    #[getset(get = "pub(crate)")]
    off:    Option<FXHelperSwitch>,
}

impl<const BOOL_ONLY: bool> FXBaseHelper<BOOL_ONLY> {
    fn allowed_literals(&self, literals: &Vec<Lit>) -> darling::Result<()> {
        if !BOOL_ONLY {
            return Ok(());
        }
        self.no_literals(literals)
    }
}

impl<const BOOL_ONLY: bool> FXHelperTrait for FXBaseHelper<BOOL_ONLY> {
    fn is_true(&self) -> bool {
        self.off.as_ref().map_or(true, |switch| !**switch)
    }

    fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }
}

impl<const BOOL_ONLY: bool> FromNestAttr for FXBaseHelper<BOOL_ONLY> {
    set_literals! {helper, ..1usize => rename as Lit::Str; pre_validate => allowed_literals}

    fn for_keyword() -> Self {
        Self::default()
    }
}

impl<const BOOL_ONLY: bool> From<FXBaseHelper<BOOL_ONLY>> for bool {
    fn from(value: FXBaseHelper<BOOL_ONLY>) -> Self {
        value.off.map_or(false, |off| off.into())
    }
}

impl<const BOOL_ONLY: bool> From<&FXBaseHelper<BOOL_ONLY>> for bool {
    fn from(value: &FXBaseHelper<BOOL_ONLY>) -> Self {
        value.off.as_ref().map_or(false, |off| off.into())
    }
}

impl<const BOOL_ONLY: bool> From<bool> for FXBaseHelper<BOOL_ONLY> {
    fn from(value: bool) -> Self {
        Self {
            rename: None,
            off:    Some(value.into()),
        }
    }
}
