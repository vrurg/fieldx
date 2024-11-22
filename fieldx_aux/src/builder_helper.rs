use crate::{
    set_literals, FXAttributes, FXBoolArg, FXHelperTrait, FXInto, FXNestingAttr, FXPubMode, FXStringArg, FXSynValue,
    FXTriggerHelper, FromNestAttr,
};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

// TODO try to issue warnings with `diagnostics` for sub-arguments which are not supported at struct or field level.
#[fxhelper]
#[derive(Debug, Default)]
pub struct FXBuilderHelper {
    #[getset(skip)]
    attributes:      Option<FXAttributes>,
    #[getset(skip)]
    attributes_impl: Option<FXAttributes>,
    #[getset(get = "pub")]
    into:            Option<FXBoolArg>,
    #[getset(get = "pub")]
    required:        Option<FXBoolArg>,
    // Means that by default a field doesn't get a builder unless explicitly specified.
    opt_in:          Option<FXBoolArg>,
    #[getset(get = "pub")]
    init:            Option<FXSynValue<syn::Ident>>,
}

impl FXBuilderHelper {
    pub fn is_into(&self) -> Option<bool> {
        self.into.as_ref().map(|i| i.is_true())
    }

    pub fn is_required(&self) -> Option<bool> {
        self.required.as_ref().map(|r| r.is_true())
    }

    pub fn is_builder_opt_in(&self) -> bool {
        self.opt_in.as_ref().map_or(false, |o| o.is_true())
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
