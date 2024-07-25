pub(crate) mod accessor;
pub(crate) mod base;
pub(crate) mod builder;
pub(crate) mod default;
#[cfg(feature = "serde")]
pub(crate) mod serde;
pub(crate) mod setter;

pub(crate) use self::{
    accessor::{FXAccessorHelper, FXAccessorMode},
    base::FXBaseHelper,
    default::FXDefault,
};
use self::{builder::FXBuilderHelper, setter::FXSetterHelper};
pub(crate) use fieldx_aux::{
    FXAttributes, FXBoolArg, FXBoolHelper, FXInto, FXNestingAttr, FXOrig, FXPubMode, FXStringArg, FXTriggerHelper,
    FromNestAttr,
};
use proc_macro2::Span;

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

pub(crate) trait FXHelperTrait: FXTriggerHelper {
    fn name(&self) -> Option<&str>;
    fn public_mode(&self) -> Option<FXPubMode>;
    fn attributes(&self) -> Option<&FXAttributes>;
    fn attributes_fn(&self) -> Option<&FXAttributes>;
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

pub(crate) type FXHelper<const BOOL_ONLY: bool = false> = FXNestingAttr<FXBaseHelper<BOOL_ONLY>>;
pub(crate) type FXAccessor<const BOOL_ONLY: bool = false> = FXNestingAttr<FXAccessorHelper<BOOL_ONLY>>;
pub(crate) type FXSetter<const BOOL_ONLY: bool = false> = FXNestingAttr<FXSetterHelper<BOOL_ONLY>>;
#[allow(dead_code)]
pub(crate) type FXBuilder = FXNestingAttr<FXBuilderHelper>;
#[cfg(feature = "serde")]
pub(crate) type FXSerde = FXNestingAttr<serde::FXSerdeHelper>;
