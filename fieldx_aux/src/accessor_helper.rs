//! Implementation of accessor helper (`get` argument of `fxstruct`/`fieldx` attributes).
use crate::FXAttributes;
use crate::FXBool;
use crate::FXOrig;
use crate::FXProp;
use crate::FXPropBool;
use crate::FXSetState;
use crate::FXString;
use crate::FXTryInto;
use crate::FromNestAttr;

use darling::util::Flag;
use darling::FromMeta;
use fieldx_derive_support::fxhelper;
use getset::Getters;
use proc_macro2::TokenStream;
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
#[fxhelper(to_tokens)]
#[derive(Default, Debug)]
pub struct FXAccessorHelper<const BOOL_ONLY: bool = false> {
    // Unfortunately, darling(flatten) over a FXAccessorMode field will break support for arguments that are implicitly
    // added by `fxhelper` attribute. Therefore we fall back to separate fields here.
    #[fxhelper(exclusive = "accessor mode")]
    clone:  Option<FXBool>,
    #[fxhelper(exclusive = "accessor mode")]
    copy:   Option<FXBool>,
    #[fxhelper(exclusive = "accessor mode")]
    as_ref: Option<FXBool>,
}

impl<const BOOL_ONLY: bool> FXAccessorHelper<BOOL_ONLY> {
    pub fn mode(&self) -> Option<FXProp<FXAccessorMode>> {
        for mode in [
            (self.clone.as_ref(), FXAccessorMode::Clone),
            (self.copy.as_ref(), FXAccessorMode::Copy),
            (self.as_ref.as_ref(), FXAccessorMode::AsRef),
        ] {
            if let Some(v) = mode.0 {
                return Some(FXProp::new(
                    if *v.is_set() { mode.1 } else { FXAccessorMode::None },
                    v.orig_span(),
                ));
            }
        }

        None
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
            if matches!(literals[0], Lit::Str(_)) {
                self.name = ("name", literals[0].clone()).fx_try_into()?;
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
