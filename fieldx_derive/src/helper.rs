pub(crate) use fieldx_aux::{FXAccessorMode, FXHelperTrait, FXOrig, FXTriggerHelper};
use proc_macro2::Span;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FXHelperKind {
    Accessor,
    AccessorMut,
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
