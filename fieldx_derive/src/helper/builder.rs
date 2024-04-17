use super::{FXAttributes, FXHelperTrait, FromNestAttr};
use crate::util::set_literals;
use darling::FromMeta;
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

#[derive(FromMeta, Debug, Clone, Default, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXFieldBuilderHelper {
    #[getset(skip)]
    rename:        Option<String>,
    attributes:    Option<FXAttributes>,
    attributes_fn: Option<FXAttributes>,
    into:          Option<bool>,
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

impl FXHelperTrait for FXFieldBuilderHelper {
    fn is_true(&self) -> bool {
        true
    }

    fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }
}
