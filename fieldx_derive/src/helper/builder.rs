use super::{FXAttributes, FXInto, FromNestAttr};
use crate::util::set_literals;
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

#[fxhelper]
#[derive(Debug, Default)]
pub(crate) struct FXBuilderHelper {
    #[getset(skip)]
    attributes:      Option<FXAttributes>,
    #[getset(skip)]
    attributes_impl: Option<FXAttributes>,
    #[getset(get = "pub(crate)")]
    into:            Option<bool>,
}

impl FXBuilderHelper {
    pub(crate) fn is_into(&self) -> Option<bool> {
        self.into
    }

    pub(crate) fn attributes_impl(&self) -> Option<&FXAttributes> {
        self.attributes_impl.as_ref()
    }
}

impl FromNestAttr for FXBuilderHelper {
    set_literals! {builder, ..1 => name as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}
