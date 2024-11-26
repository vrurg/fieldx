use crate::{
    set_literals, FXAttributes, FXBoolArg, FXHelperTrait, FXInto, FXNestingAttr, FXOrig, FXPubMode, FXPunctuated,
    FXStringArg, FXSynValue, FXTriggerHelper, FromNestAttr,
};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use proc_macro2::Span;
use syn::{spanned::Spanned, Lit, Token};

// TODO try to issue warnings with `diagnostics` for sub-arguments which are not supported at struct or field level.
#[fxhelper(validate = Self::validate)]
#[derive(Debug, Default, Getters)]
#[getset(get = "pub")]
pub struct FXBuilderHelper<const STRUCT: bool = false> {
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
    post_build:      Option<FXSynValue<syn::Ident, true>>,
    #[getset(get = "pub")]
    error:           Option<FXSynValue<FXPunctuated<syn::Path, Token![,], 1, 2>>>,
}

impl<const STRUCT: bool> FXBuilderHelper<STRUCT> {
    pub fn is_into(&self) -> Option<bool> {
        self.into.as_ref().map(|i| i.is_true())
    }

    pub fn is_required(&self) -> Option<bool> {
        self.required.as_ref().map(|r| r.is_true())
    }

    pub fn is_builder_opt_in(&self) -> bool {
        self.opt_in.as_ref().map_or(false, |o| o.is_true())
    }

    pub fn has_post_build(&self) -> bool {
        self.post_build.is_some()
    }

    pub fn attributes_impl(&self) -> Option<&FXAttributes> {
        self.attributes_impl.as_ref()
    }

    pub fn error_type(&self) -> Option<&syn::Path> {
        self.error().as_ref().and_then(|ev| ev.items().first())
    }

    pub fn error_variant(&self) -> Option<&syn::Path> {
        self.error().as_ref().and_then(|ev| ev.items().get(1))
    }

    pub fn validate(&self) -> darling::Result<()> {
        if !STRUCT && self.error.is_some() {
            return Err(
                darling::Error::custom(format!("parameter 'error' is only supported at struct level")).with_span(
                    &self
                        .error
                        .as_ref()
                        .unwrap()
                        .orig()
                        .map_or_else(|| Span::call_site(), |o| o.span()),
                ),
            );
        }
        Ok(())
    }
}

impl<const STRUCT: bool> FromNestAttr for FXBuilderHelper<STRUCT> {
    set_literals! {builder, ..1 => name as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}
