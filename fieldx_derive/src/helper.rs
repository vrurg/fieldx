pub(crate) mod accessor;
pub(crate) mod attributes;
pub(crate) mod base;
pub(crate) mod builder;
pub(crate) mod nesting_attr;
pub(crate) mod setter;
pub(crate) mod with_origin;

pub(crate) use self::{
    accessor::{FXAccessorHelper, FXAccessorMode},
    attributes::FXAttributes,
    base::FXBaseHelper,
    builder::FXArgsBuilderHelper,
    nesting_attr::{FXNestingAttr, FromNestAttr},
    with_origin::{FXOrig, FXWithOrig},
};
use self::{builder::FXFieldBuilderHelper, setter::FXSetterHelper};
use darling::FromMeta;
use getset::Getters;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::ops::Deref;
use syn::{Lit, Meta};

pub(crate) trait FXHelperTrait {
    fn is_true(&self) -> bool;
    fn rename(&self) -> Option<&str>;
}

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXHelperSwitch {
    flag: bool,
    orig: Option<Meta>,
}

#[derive(FromMeta, Debug, Clone, Default)]
pub(crate) enum FXPubMode {
    Crate,
    Super,
    InMod(syn::Path),
    #[default]
    #[darling(skip)]
    All,
}

impl FromMeta for FXHelperSwitch {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        if let Meta::Path(ref path) = &item {
            if path.is_ident("true") || path.is_ident("on") || path.is_ident("yes") {
                return Ok(Self {
                    flag: true,
                    orig: Some(item.clone()),
                });
            }
            else if path.is_ident("false") || path.is_ident("off") || path.is_ident("no") {
                return Ok(Self {
                    flag: false,
                    orig: Some(item.clone()),
                });
            }
        }
        let err =
            darling::Error::custom(format!("Unsupported expression '{}'", item.to_token_stream())).with_span(item);
        #[cfg(feature = "diagnostics")]
        err.note("Expected either one of: 'true', 'on', 'yes', 'false', 'off', or 'no'");
        Err(err)
    }

    fn from_none() -> Option<Self> {
        Some(Self {
            flag: false,
            orig: None,
        })
    }
}

impl From<FXHelperSwitch> for bool {
    fn from(value: FXHelperSwitch) -> Self {
        value.flag
    }
}

impl From<&FXHelperSwitch> for bool {
    fn from(value: &FXHelperSwitch) -> Self {
        value.flag
    }
}

impl From<bool> for FXHelperSwitch {
    fn from(value: bool) -> Self {
        Self {
            flag: value,
            orig: None,
        }
    }
}

impl ToTokens for FXHelperSwitch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(ref orig) = self.orig {
            orig.to_tokens(tokens);
        }
    }
}

impl FXOrig<syn::Meta> for FXHelperSwitch {
    fn orig(&self) -> Option<&syn::Meta> {
        self.orig.as_ref()
    }
}

impl Deref for FXHelperSwitch {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.flag
    }
}

impl FXPubMode {
    pub(crate) fn vis_tok(&self) -> TokenStream {
        match self {
            Self::All => quote![pub],
            Self::Super => quote![pub(super)],
            Self::Crate => quote![pub(crate)],
            Self::InMod(ref path) => quote![pub(in #path)],
        }
    }
}

impl FromNestAttr for FXPubMode {
    fn for_keyword() -> Self {
        Self::All
    }

    fn set_literals(self, _literals: &Vec<Lit>) -> darling::Result<Self> {
        Err(darling::Error::custom("No literals allowed here"))
    }
}

pub(crate) type FXHelper<const BOOL_ONLY: bool = false> = FXNestingAttr<FXBaseHelper<BOOL_ONLY>>;
pub(crate) type FXAccessor<const BOOL_ONLY: bool = false> = FXNestingAttr<FXAccessorHelper<BOOL_ONLY>>;
pub(crate) type FXSetter<const BOOL_ONLY: bool = false> = FXNestingAttr<FXSetterHelper<BOOL_ONLY>>;
pub(crate) type FXArgsBuilder = FXNestingAttr<FXArgsBuilderHelper>;
pub(crate) type FXFieldBuilder = FXNestingAttr<FXFieldBuilderHelper>;
