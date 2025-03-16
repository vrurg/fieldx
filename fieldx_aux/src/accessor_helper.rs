//! Implementation of accessor helper (`get` argument of `fxstruct`/`fieldx` attributes).
use crate::{FXAttributes, FXBool, FXInto, FXOrig, FXProp, FXSetState, FXString, FXTriggerHelper, FromNestAttr};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use quote::ToTokens;
use syn::Lit;

/// Accessor mode defines the type it returns.
#[derive(FromMeta, Debug, Clone, Copy, Default, PartialEq)]
pub enum FXAccessorMode {
    /// Return a copy of the value for types implementing the [`Copy`] trait.
    Copy,
    /// Return a clone of the value for types implementing the [`Clone`] trait.
    Clone,
    /// apply `.as_ref()` method to optional fields. I.e. return `Option<&ValueType>`.
    AsRef,
    #[default]
    #[darling(skip)]
    None,
}

impl FXAccessorMode {
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    #[inline]
    pub fn is_copy(&self) -> bool {
        matches!(self, Self::Copy)
    }

    #[inline]
    pub fn is_clone(&self) -> bool {
        matches!(self, Self::Clone)
    }

    #[inline]
    pub fn is_as_ref(&self) -> bool {
        matches!(self, Self::AsRef)
    }
}

impl FXSetState for FXAccessorMode {
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(!self.is_none(), None)
    }
}

/// Implement support for accessor attribute argument.
#[fxhelper]
#[derive(Default, Debug)]
pub struct FXAccessorHelper<const BOOL_ONLY: bool = false> {
    // Unfortunately, darling(flatten) over a FXAccessorMode field will break support for arguments that are implicitly
    // added by `fxhelper` attribute. Therefore we fall back to separate fields here.
    #[fxhelper(exclusive = "accessor mode")]
    clone:  Flag,
    #[fxhelper(exclusive = "accessor mode")]
    copy:   Flag,
    #[fxhelper(exclusive = "accessor mode")]
    as_ref: Flag,
}

impl<const BOOL_ONLY: bool> FXAccessorHelper<BOOL_ONLY> {
    pub fn mode(&self) -> Option<FXProp<FXAccessorMode>> {
        Some(if self.clone.is_present() {
            FXProp::new(FXAccessorMode::Clone, Some(self.clone.span()))
        }
        else if self.copy.is_present() {
            FXProp::new(FXAccessorMode::Copy, Some(self.copy.span()))
        }
        else if self.as_ref.is_present() {
            FXProp::new(FXAccessorMode::AsRef, Some(self.as_ref.span()))
        }
        else {
            return None;
        })
    }
}

impl<const BOOL_ONLY: bool> FromNestAttr for FXAccessorHelper<BOOL_ONLY> {
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
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
                self.name = Some(str.value().fx_into());
            }
            else {
                let err =
                    darling::Error::unexpected_type(&literals[0].to_token_stream().to_string()).with_span(&literals[0]);
                #[cfg(feature = "diagnostics")]
                let err = err.note("Expected a string with helper name");
                return Err(err);
            }
        }
        Ok(self)
    }
}
