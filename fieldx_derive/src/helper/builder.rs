use super::{FXAttributes, FromNestAttr};
use crate::util::set_literals;
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::Lit;

#[derive(FromMeta, Debug, Clone, Default, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXArgsBuilderHelper {
    attributes:      Option<FXAttributes>,
    attributes_impl: Option<FXAttributes>,
    into:            Option<bool>,
}

impl FXArgsBuilderHelper {
    pub(crate) fn is_into(&self) -> Option<bool> {
        self.into
    }
}

impl FromNestAttr for FXArgsBuilderHelper {
    set_literals! {builder}

    fn for_keyword() -> Self {
        Default::default()
    }
}

#[fxhelper]
#[derive(Debug, Default)]
pub(crate) struct FXFieldBuilderHelper {
    #[getset(skip)]
    attributes: Option<FXAttributes>,
    #[getset(get = "pub(crate)")]
    into:       Option<bool>,
}

impl FXFieldBuilderHelper {
    pub(crate) fn is_into(&self) -> Option<bool> {
        self.into
    }
}

impl FromNestAttr for FXFieldBuilderHelper {
    set_literals! {builder, ..1 => rename as Lit::Str}

    fn for_keyword() -> Self {
        Default::default()
    }
}

// impl FXHelperTrait for FXFieldBuilderHelper {
//     fn is_true(&self) -> bool {
//         !self.off.is_present()
//     }

//     fn rename(&self) -> Option<&str> {
//         self.rename.as_deref()
//     }

//     fn attributes(&self) -> Option<&FXAttributes> {
//         self.attributes.as_ref()
//     }

//     fn attributes_fn(&self) -> Option<&FXAttributes> {
//         self.attributes_fn.as_ref()
//     }
// }
