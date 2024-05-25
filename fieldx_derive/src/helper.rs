pub(crate) mod accessor;
pub(crate) mod attributes;
pub(crate) mod base;
pub(crate) mod builder;
pub(crate) mod default;
pub(crate) mod nesting_attr;
#[cfg(feature = "serde")]
pub(crate) mod serde;
pub(crate) mod setter;
pub(crate) mod value;
pub(crate) mod with_origin;

pub(crate) use self::{
    accessor::{FXAccessorHelper, FXAccessorMode},
    attributes::FXAttributes,
    base::FXBaseHelper,
    default::FXDefault,
    nesting_attr::{FXNestingAttr, FromNestAttr},
    value::FXValueArg,
    with_origin::FXWithOrig,
};
use self::{builder::FXBuilderHelper, setter::FXSetterHelper};
use darling::FromMeta;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::Lit;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FXHelperKind {
    AccessorMut,
    Accessor,
    Builder,
    Clearer,
    Lazy,
    Predicate,
    Reader,
    Setter,
    Writer,
}

pub(crate) trait FXHelperContainer {
    fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait>;
    fn get_helper_span(&self, kind: FXHelperKind) -> Option<Span>;
}

pub(crate) trait FXTriggerHelper {
    fn is_true(&self) -> bool;
}

pub(crate) trait FXBoolHelper {
    fn is_true(&self) -> bool;
    fn is_true_opt(&self) -> Option<bool>;
    fn not_true(&self) -> bool {
        !self.is_true()
    }
    // fn not_true_opt(&self) -> Option<bool> {
    //     self.is_true_opt().map(|b| !b)
    // }
}

pub(crate) trait FXHelperTrait: FXTriggerHelper {
    fn name(&self) -> Option<&str>;
    fn public_mode(&self) -> Option<FXPubMode>;
    fn attributes(&self) -> Option<&FXAttributes>;
    fn attributes_fn(&self) -> Option<&FXAttributes>;
}

pub(crate) trait FXFrom<T> {
    fn fx_from(value: T) -> Self;
}

pub(crate) trait FXInto<T> {
    fn fx_into(self) -> T;
}

#[derive(FromMeta, Debug, Clone, Default)]
pub(crate) enum FXPubMode {
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

impl<T, U> FXInto<U> for T
where
    U: FXFrom<T>,
{
    #[inline]
    fn fx_into(self) -> U {
        U::fx_from(self)
    }
}

impl ToString for FXHelperKind {
    fn to_string(&self) -> String {
        match self {
            FXHelperKind::Accessor => "accessor",
            FXHelperKind::AccessorMut => "accessor_mut",
            FXHelperKind::Builder => "builder setter",
            FXHelperKind::Clearer => "clearer",
            FXHelperKind::Lazy => "lazy builder",
            FXHelperKind::Predicate => "predicate",
            FXHelperKind::Reader => "reader",
            FXHelperKind::Setter => "setter",
            FXHelperKind::Writer => "writer",
        }
        .to_string()
    }
}

impl FXHelperKind {
    #[inline]
    pub(crate) fn default_prefix(&self) -> Option<&str> {
        match self {
            FXHelperKind::AccessorMut => None,
            FXHelperKind::Accessor => None,
            FXHelperKind::Builder => None,
            FXHelperKind::Clearer => Some("clear_"),
            FXHelperKind::Lazy => Some("build_"),
            FXHelperKind::Predicate => Some("has_"),
            FXHelperKind::Reader => Some("read_"),
            FXHelperKind::Setter => Some("set_"),
            FXHelperKind::Writer => Some("write_"),
        }
        .into()
    }

    #[inline]
    pub(crate) fn default_suffix(&self) -> Option<&str> {
        match self {
            FXHelperKind::AccessorMut => Some("_mut"),
            FXHelperKind::Accessor => None,
            FXHelperKind::Builder => None,
            FXHelperKind::Clearer => None,
            FXHelperKind::Lazy => None,
            FXHelperKind::Predicate => None,
            FXHelperKind::Reader => None,
            FXHelperKind::Setter => None,
            FXHelperKind::Writer => None,
        }
    }
}

impl<H: FXTriggerHelper> FXBoolHelper for Option<H> {
    #[inline]
    fn is_true(&self) -> bool {
        self.as_ref().map_or(false, |h| h.is_true())
    }

    #[inline]
    fn is_true_opt(&self) -> Option<bool> {
        self.as_ref().map(|h| h.is_true())
    }
}

pub(crate) type FXHelper<const BOOL_ONLY: bool = false> = FXNestingAttr<FXBaseHelper<BOOL_ONLY>>;
pub(crate) type FXAccessor<const BOOL_ONLY: bool = false> = FXNestingAttr<FXAccessorHelper<BOOL_ONLY>>;
pub(crate) type FXSetter<const BOOL_ONLY: bool = false> = FXNestingAttr<FXSetterHelper<BOOL_ONLY>>;
#[allow(dead_code)]
pub(crate) type FXValue<T, const BOOL_ONLY: bool = false> = FXNestingAttr<FXValueArg<T, BOOL_ONLY>>;
pub(crate) type FXStringArg = FXNestingAttr<FXValueArg<String>>;
pub(crate) type FXBoolArg = FXNestingAttr<FXValueArg<(), true>>;
pub(crate) type FXBuilder = FXNestingAttr<FXBuilderHelper>;
#[cfg(feature = "serde")]
pub(crate) type FXSerde = FXNestingAttr<serde::FXSerdeHelper>;
