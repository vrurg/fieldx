//! Literal value arguments

use crate::traits::FXSetState;
use crate::FXFrom;
use crate::FXOrig;
use crate::FXProp;
use crate::FXSynValueArg;
use crate::FXTryFrom;
use crate::FromNestAttr;
use darling::util::Flag;
use darling::FromMeta;
use proc_macro2::Span;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use syn::Lit;

#[derive(Debug, Clone, Copy)]
pub struct FXEmpty;

impl ToTokens for FXEmpty {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {}
}

/// Represents arguments that take a single literal value or serve as an explicit flag.
///
/// When the `BOOL_ONLY` parameter is set to `true`, the argument serves only as a trigger.
///
/// For example, you might see `pi(3.1415926)`, `pi`, `pi(off, 3.1415926)`, or `pi(off)`. The advantage of the latter
/// form is that, unlike [`darling::util::Flag`], it is explicit, making it visually clear what’s happening.  This is
/// especially useful for debugging when you need to temporarily disable something.
#[derive(Debug, FromMeta, Clone)]
pub struct FXValueArg<T, const BOOL_ONLY: bool = false> {
    off:   Flag,
    #[darling(skip)]
    value: Option<T>,
    #[darling(skip)]
    orig:  Option<syn::Lit>,
}

impl<T, const BOOL_ONLY: bool> FXValueArg<T, BOOL_ONLY> {
    fn validate_literals(literals: &Vec<Lit>) -> darling::Result<()> {
        if literals.len() > 0 && BOOL_ONLY {
            Err(darling::Error::custom("No literal arguments are allowed here").with_span(&literals[0].span()))
        }
        else if literals.len() > 1 {
            Err(darling::Error::custom("Only one literal argument is allowed here").with_span(&literals[0].span()))
        }
        else {
            Ok(())
        }
    }

    fn as_keyword(path: &syn::Path) -> darling::Result<Self> {
        if BOOL_ONLY {
            Ok(Self {
                off:   Flag::default(),
                value: None,
                orig:  None,
            })
        }
        else {
            Err(darling::Error::custom(format!("A literal {} argument is required", stringify!(T))).with_span(path))
        }
    }

    pub fn value(&self) -> Option<&T> {
        if *self.is_set() {
            self.value.as_ref()
        }
        else {
            None
        }
    }
}

impl<T: ToTokens, const BOOL_ONLY: bool> FXValueArg<T, BOOL_ONLY> {
    pub fn set_span(&mut self, span: Span) {
        if let Some(ref mut orig) = self.orig {
            orig.set_span(span);
        }
        else if self.value.is_some() {
            let val = self.value.as_ref().unwrap();
            self.orig = Some(
                syn::parse2::<syn::ExprLit>(quote_spanned! {span=> #val })
                    .expect("Failed to parse literal as syn::ExprLit")
                    .lit,
            );
        }
    }
}

impl<T: ToTokens, const BOOL_ONLY: bool> ToTokens for FXValueArg<T, BOOL_ONLY> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut parts = vec![];
        if self.off.is_present() {
            parts.push(quote! { off });
        }
        if let Some(ref orig) = self.orig {
            parts.push(orig.to_token_stream());
        }
        else if let Some(ref value) = self.value {
            parts.push(quote! { #value });
        }
        tokens.extend(quote! { #(#parts),* });
    }
}

impl<T, const BOOL_ONLY: bool> FXOrig<syn::Lit> for FXValueArg<T, BOOL_ONLY> {
    fn orig(&self) -> Option<&syn::Lit> {
        self.orig.as_ref()
    }
}

impl<T, const BOOL_ONLY: bool> FXSetState for FXValueArg<T, BOOL_ONLY> {
    fn is_set(&self) -> FXProp<bool> {
        if self.off.is_present() {
            FXProp::new(false, Some(self.off.span()))
        }
        else {
            // If this value is a flag, i.e. it may only be set or unset with `off`, then we should return true in this
            // branch.
            FXProp::new(BOOL_ONLY || self.value.is_some(), None)
        }
    }

    // Optimize by bypassing the `FXProp` wrapper.
    fn is_set_bool(&self) -> bool {
        self.off.is_present() || BOOL_ONLY || self.value.is_some()
    }
}

impl<T, const BOOL_ONLY: bool> From<T> for FXValueArg<T, BOOL_ONLY> {
    fn from(value: T) -> Self {
        Self {
            off:   Flag::default(),
            value: Some(value),
            orig:  None,
        }
    }
}

impl FromNestAttr for FXValueArg<FXEmpty, true> {
    fn set_literals(self, literals: &Vec<Lit>) -> darling::Result<Self> {
        Self::validate_literals(literals)?;
        Ok(self)
    }

    fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
        Self::as_keyword(path)
    }
}

impl<T> FromNestAttr for FXValueArg<FXSynValueArg<T>, false> {
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self {
            off:   Flag::default(),
            value: None,
            orig:  None,
        })
    }
}

impl<T, const BOOL_ONLY: bool> FXFrom<T> for FXValueArg<T, BOOL_ONLY> {
    fn fx_from(value: T) -> Self {
        Self {
            off:   Flag::default(),
            value: Some(value),
            orig:  None,
        }
    }
}

impl<T, const BOOL_ONLY: bool> FXFrom<Option<T>> for FXValueArg<T, BOOL_ONLY> {
    fn fx_from(value: Option<T>) -> Self {
        Self {
            off: Flag::default(),
            value,
            orig: None,
        }
    }
}

impl FXFrom<syn::LitStr> for FXValueArg<String> {
    fn fx_from(value: syn::LitStr) -> Self {
        Self {
            off:   Flag::default(),
            value: Some(value.value()),
            orig:  Some(value.clone().into()),
        }
    }
}

impl FXTryFrom<syn::Lit> for FXValueArg<String> {
    type Error = darling::Error;

    fn fx_try_from(value: syn::Lit) -> Result<Self, Self::Error> {
        if let syn::Lit::Str(lit) = value.clone() {
            Ok(Self {
                off:   Flag::from(false),
                value: Some(lit.value()),
                orig:  Some(value),
            })
        }
        else {
            Err(darling::Error::unexpected_lit_type(&value))
        }
    }
}

impl FXTryFrom<syn::Lit> for Option<FXValueArg<String>> {
    type Error = darling::Error;

    fn fx_try_from(value: syn::Lit) -> Result<Self, Self::Error> {
        if let syn::Lit::Str(lit) = value.clone() {
            Ok(Some(FXValueArg {
                off:   Flag::from(false),
                value: Some(lit.value()),
                orig:  Some(value),
            }))
        }
        else {
            Err(darling::Error::unexpected_lit_type(&value))
        }
    }
}

macro_rules! from_nest_attr_num {
    ( $($from:path => $ty:ty);+ $(;)? ) => {
        $(from_nest_attr_num!(@ $from => $ty);)+
    };
    (@ $from:path => $ty:ty) => {
        impl $crate::FromNestAttr for FXValueArg<$ty, false> {
            fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
                Self::validate_literals(literals)?;
                if let $from(ref lit) = literals[0] {
                    self.orig = Some(literals[0].clone());
                    self.value = Some(lit.base10_parse()?);
                }
                else {
                    return Err(darling::Error::unexpected_lit_type(&literals[0]));
                }
                Ok(self)
            }

            fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
                Self::as_keyword(path)
            }
        }
    };
}

macro_rules! from_nest_attr_val {
    ( $( $from:path => $ty:ty );+ $(;)? ) => {
        $(from_nest_attr_val!(@ $from => $ty);)+
    };
    (@ $from:path => $ty:ty) => {
        impl $crate::FromNestAttr for FXValueArg<$ty, false> {
            fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
                Self::validate_literals(literals)?;
                if let $from(ref lit) = literals[0] {
                    self.orig = Some(literals[0].clone());
                    self.value = Some(lit.value());
                }
                else {
                    return Err(darling::Error::unexpected_lit_type(&literals[0]));
                }
                Ok(self)
            }

            fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
                Self::as_keyword(path)
            }
        }
    };
}

from_nest_attr_num! {
    Lit::Int => i8;
    Lit::Int => i16;
    Lit::Int => i32;
    Lit::Int => i64;
    Lit::Int => u8;
    Lit::Int => u16;
    Lit::Int => u32;
    Lit::Int => u64;
    Lit::Float => f32;
    Lit::Float => f64;
}
from_nest_attr_val! {
    Lit::Str => String;
    Lit::Char => char;
    Lit::Bool => bool;
}
