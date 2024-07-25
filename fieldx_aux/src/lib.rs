pub mod attributes;
pub mod nesting_attr;
pub mod traits;
pub mod value;
pub mod with_origin;

pub use crate::{
    attributes::FXAttributes,
    nesting_attr::{FXNestingAttr, FromNestAttr},
    traits::{FXBoolHelper, FXFrom, FXInto, FXTriggerHelper},
    value::FXValueArg,
    with_origin::FXOrig,
};
use darling::FromMeta;
use quote::{quote, ToTokens};
use syn::Lit;

#[derive(FromMeta, Debug, Clone, Default)]
pub enum FXPubMode {
    #[darling(skip)]
    Private,
    Crate,
    Super,
    InMod(syn::Path),
    #[default]
    #[darling(skip)]
    All,
}

impl FromNestAttr for FXPubMode {
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::All)
    }

    fn set_literals(self, _literals: &Vec<Lit>) -> darling::Result<Self> {
        Err(darling::Error::custom("No literals allowed here"))
    }
}

impl ToTokens for FXPubMode {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            FXPubMode::Private => quote![],
            FXPubMode::All => quote!(pub),
            FXPubMode::Super => quote!(pub(super)),
            FXPubMode::Crate => quote!(pub(crate)),
            FXPubMode::InMod(ref path) => quote!(pub(in #path)),
        });
    }
}

pub type FXValue<T, const BOOL_ONLY: bool = false> = FXNestingAttr<FXValueArg<T, BOOL_ONLY>>;
pub type FXStringArg = FXNestingAttr<FXValueArg<String>>;
pub type FXBoolArg = FXNestingAttr<FXValueArg<(), true>>;
