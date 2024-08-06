use crate::{
    set_literals, FXAttributes, FXBoolArg, FXHelperTrait, FXInto, FXNestingAttr, FXPubMode, FXStringArg,
    FXTriggerHelper, FromNestAttr,
};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

#[fxhelper]
#[derive(Debug, Default)]
pub struct FXBuilderHelper {
    #[getset(skip)]
    attributes:      Option<FXAttributes>,
    #[getset(skip)]
    attributes_impl: Option<FXAttributes>,
    #[getset(get = "pub")]
    into:            Option<bool>,
}

impl FXBuilderHelper {
    pub fn is_into(&self) -> Option<bool> {
        self.into
    }

    pub fn attributes_impl(&self) -> Option<&FXAttributes> {
        self.attributes_impl.as_ref()
    }
}

impl FromNestAttr for FXBuilderHelper {
    set_literals! {builder, ..1 => name as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}
