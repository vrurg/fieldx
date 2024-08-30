pub mod accessor_helper;
pub mod attributes;
pub mod base_helper;
pub mod builder_helper;
pub mod default_helper;
pub mod nesting_attr;
#[cfg(feature = "serde")]
pub mod serde_helper;
pub mod setter_helper;
pub mod traits;
pub mod util;
pub mod value;
pub mod with_origin;

#[cfg(feature = "serde")]
pub use crate::serde_helper::FXSerdeHelper;
pub use crate::{
    accessor_helper::{FXAccessorHelper, FXAccessorMode},
    attributes::FXAttributes,
    base_helper::FXBaseHelper,
    builder_helper::FXBuilderHelper,
    default_helper::FXDefault,
    nesting_attr::{FXNestingAttr, FromNestAttr},
    setter_helper::FXSetterHelper,
    traits::{FXBoolHelper, FXFrom, FXHelperTrait, FXInto, FXTriggerHelper},
    util::public_mode,
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

impl FXTriggerHelper for FXPubMode {
    fn is_true(&self) -> bool {
        true
    }
}

pub type FXHelper<const BOOL_ONLY: bool = false> = FXNestingAttr<FXBaseHelper<BOOL_ONLY>>;
pub type FXValue<T, const BOOL_ONLY: bool = false> = FXNestingAttr<FXValueArg<T, BOOL_ONLY>>;
pub type FXStringArg = FXNestingAttr<FXValueArg<String>>;
pub type FXBoolArg = FXNestingAttr<FXValueArg<(), true>>;
pub type FXAccessor<const BOOL_ONLY: bool = false> = FXNestingAttr<FXAccessorHelper<BOOL_ONLY>>;
pub type FXSetter<const BOOL_ONLY: bool = false> = FXNestingAttr<FXSetterHelper<BOOL_ONLY>>;
#[allow(dead_code)]
pub type FXBuilder = FXNestingAttr<FXBuilderHelper>;
#[cfg(feature = "serde")]
pub type FXSerde = FXNestingAttr<FXSerdeHelper>;
