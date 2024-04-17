use super::{FXAttributes, FXHelperTrait, FromNestAttr};
use darling::{util::Flag, FromMeta};
use quote::ToTokens;
use syn::Lit;

#[derive(FromMeta, Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum FXAccessorMode {
    Copy,
    Clone,
    #[default]
    #[darling(skip)]
    None,
}

#[derive(FromMeta, Default, Debug, Clone)]
pub(crate) struct FXAccessorHelper<const BOOL_ONLY: bool = false> {
    rename: Option<String>,
    off:    Flag,
    attributes_fn: Option<FXAttributes>,
    #[darling(flatten, default)]
    mode:   FXAccessorMode,
}

impl<const BOOL_ONLY: bool> FXAccessorHelper<BOOL_ONLY> {
    pub(crate) fn mode(&self) -> Option<&FXAccessorMode> {
        if self.mode == FXAccessorMode::None {
            None
        }
        else {
            Some(&self.mode)
        }
    }
}

impl<const BOOL_ONLY: bool> FXHelperTrait for FXAccessorHelper<BOOL_ONLY> {
    fn is_true(&self) -> bool {
        !self.off.is_present()
    }

    fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    fn attributes_fn(&self) -> Option<&FXAttributes> {
        self.attributes_fn.as_ref()
    }
}

impl<const BOOL_ONLY: bool> FromNestAttr for FXAccessorHelper<BOOL_ONLY> {
    fn for_keyword() -> Self {
        Self::default()
    }

    fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
        if BOOL_ONLY {
            return Err(darling::Error::custom("Literal values are not supported here").with_span(&literals[0]));
        }

        if literals.len() > 1 {
            return Err(darling::Error::custom("Too many literals"));
        }
        else if literals.len() == 1 {
            if let Lit::Str(ref str) = literals[0] {
                self.rename = Some(str.value());
            }
            else {
                let err =
                    darling::Error::unexpected_type(&literals[0].to_token_stream().to_string()).with_span(&literals[0]);
                #[cfg(feature = "diagnostics")]
                err.note("Expected a string with helper name");
                return Err(err);
            }
        }
        Ok(self)
    }
}
