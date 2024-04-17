use super::{FXHelperSwitch, FXHelperTrait, FromNestAttr};
use crate::util::set_literals;
use darling::FromMeta;
use getset::Getters;
use syn::Lit;

#[derive(FromMeta, Default, Debug, Clone, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXSetterHelper<const BOOL_ONLY: bool = false> {
    #[getset(skip)]
    rename: Option<String>,
    into:   Option<bool>,
    off:    Option<FXHelperSwitch>,
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

impl<const BOOL_ONLY: bool> FXHelperTrait for FXSetterHelper<BOOL_ONLY> {
    fn is_true(&self) -> bool {
        self.off.as_ref().map_or(true, |switch| !**switch)
    }

    fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }
}

impl<const BOOL_ONLY: bool> FromNestAttr for FXSetterHelper<BOOL_ONLY> {
    set_literals! { setter, ..1 => rename as Lit::Str; pre_validate => allowed_literals }

    fn for_keyword() -> Self {
        Self::default()
    }
}
