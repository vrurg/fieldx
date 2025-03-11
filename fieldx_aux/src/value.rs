//! Literal value arguments

use crate::{traits::FXSetState, FXFrom, FXProp, FXSynValueArg, FXTriggerHelper, FXTryFrom, FromNestAttr};
use darling::{util::Flag, FromMeta};
use syn::Lit;

/// Arguments that take single literal value or can serve as explicit flag.
///
/// If the `BOOL_ONLY` parameter is set to true then the argument can serve as a trigger only.
///
/// So, we either can see something like `pi(3.1415926)`, `pi`, `pi(off, 3.1415926)` or `pi(off)`. The advantage of the
/// latter form is that, contrary to [`darling::util::Flag`], it is explicit making it visually clear what's going on.
/// It is specially useful for debugging when one would just need to temporarily disable something.
#[derive(Debug, FromMeta, Clone)]
pub struct FXValueArg<T, const BOOL_ONLY: bool = false> {
    off:   Flag,
    #[darling(skip)]
    value: Option<T>,
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
            })
        }
        else {
            Err(darling::Error::custom(format!("A literal {} argument is required", stringify!(T))).with_span(path))
        }
    }

    pub fn value(&self) -> Option<&T> {
        if *self.is_true() {
            self.value.as_ref()
        }
        else {
            None
        }
    }
}

impl<T, const BOOL_ONLY: bool> FXTriggerHelper for FXValueArg<T, BOOL_ONLY> {
    fn is_true(&self) -> FXProp<bool> {
        if self.off.is_present() {
            FXProp::new(false, Some(self.off.span()))
        }
        else {
            true.into()
        }
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
}

impl<T, const BOOL_ONLY: bool> From<T> for FXValueArg<T, BOOL_ONLY> {
    fn from(value: T) -> Self {
        Self {
            off:   Flag::default(),
            value: Some(value),
        }
    }
}

impl FromNestAttr for FXValueArg<(), true> {
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
        })
    }
}

impl<T, const BOOL_ONLY: bool> FXFrom<T> for FXValueArg<T, BOOL_ONLY> {
    fn fx_from(value: T) -> Self {
        Self {
            value: Some(value),
            off:   Flag::default(),
        }
    }
}

impl<T, const BOOL_ONLY: bool> FXFrom<Option<T>> for FXValueArg<T, BOOL_ONLY> {
    fn fx_from(value: Option<T>) -> Self {
        Self {
            value,
            off: Flag::default(),
        }
    }
}

impl FXFrom<syn::LitStr> for FXValueArg<String> {
    fn fx_from(value: syn::LitStr) -> Self {
        Self {
            value: Some(value.value()),
            off:   Flag::default(),
        }
    }
}

impl FXTryFrom<syn::Lit> for FXValueArg<String> {
    type Error = darling::Error;

    fn fx_try_from(value: syn::Lit) -> Result<Self, Self::Error> {
        if let syn::Lit::Str(lit) = value {
            Ok(Self {
                value: Some(lit.value()),
                off:   Flag::from(false),
            })
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
        impl crate::FromNestAttr for FXValueArg<$ty, false> {
            fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
                Self::validate_literals(literals)?;
                if let $from(ref lit) = literals[0] {
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
        impl crate::FromNestAttr for FXValueArg<$ty, false> {
            fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
                Self::validate_literals(literals)?;
                if let $from(ref lit) = literals[0] {
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
