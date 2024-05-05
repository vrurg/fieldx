use darling::{FromField, FromMeta};
use proc_macro2::Span;
use std::ops::Deref;
use syn::{self, spanned::Spanned};

pub trait FXOrig<O>
where
    O: Spanned,
{
    #[allow(dead_code)]
    fn orig(&self) -> Option<&O>;

    #[allow(dead_code)]
    fn span(&self) -> Option<Span> {
        self.orig().and_then(|o| Some(o.span()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FXWithOrig<T, O> {
    parsed: T,
    orig:   Option<O>,
}

impl<T, O> FXWithOrig<T, O> {
    pub(crate) fn new(parsed: T, orig: O) -> Self {
        Self {
            parsed,
            orig: Some(orig),
        }
    }
}

impl<T, O: Spanned> FXOrig<O> for FXWithOrig<T, O> {
    fn orig(&self) -> Option<&O> {
        self.orig.as_ref()
    }
}

impl<T, O: Spanned> Deref for FXWithOrig<T, O> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

impl<T, O: Spanned> AsRef<T> for FXWithOrig<T, O> {
    fn as_ref(&self) -> &T {
        &self.parsed
    }
}

impl<T, O: Spanned> AsMut<T> for FXWithOrig<T, O> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.parsed
    }
}

macro_rules! fx_with_orig {
    ($for_trait:ident, $func:ident, $syn:path) => {
        impl<T: $for_trait> $for_trait for FXWithOrig<T, $syn> {
            fn $func(value: &$syn) -> ::darling::Result<Self> {
                Ok(FXWithOrig::new($for_trait::$func(value)?, value.clone()))
            }
        }
    };
}

fx_with_orig! {FromMeta, from_meta, syn::Meta}
fx_with_orig! {FromField, from_field, syn::Field}
