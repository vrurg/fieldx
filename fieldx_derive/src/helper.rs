pub(crate) mod accessor;
pub(crate) mod attributes;
pub(crate) mod base;
pub(crate) mod builder;
pub(crate) mod nesting_attr;
#[cfg(feature = "serde")]
pub(crate) mod serde;
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
use syn::Lit;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FXHelperKind {
    Accessor,
    AccesorMut,
    Setter,
    Reader,
    Writer,
    Clearer,
    Predicate,
}

pub(crate) trait FXHelperContainer {
    fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait>;
}

pub(crate) trait FXHelperTrait {
    fn is_true(&self) -> bool;
    fn rename(&self) -> Option<&str>;
    fn public_mode(&self) -> Option<FXPubMode>;
    fn attributes(&self) -> Option<&FXAttributes>;
    fn attributes_fn(&self) -> Option<&FXAttributes>;
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
    fn for_keyword() -> darling::Result<Self> {
        Ok(Self::All)
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
#[cfg(feature = "serde")]
pub(crate) type FXSerde = FXNestingAttr<serde::FXSerdeHelper>;
