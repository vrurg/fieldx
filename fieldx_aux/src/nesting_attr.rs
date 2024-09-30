use crate::{with_origin::FXOrig, FXFrom, FXTriggerHelper};
use darling::{ast::NestedMeta, FromMeta};
use getset::Getters;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::ops::{Deref, DerefMut};
use syn::{Lit, Meta};

pub trait FromNestAttr<const WITH_LITERALS: bool = true>: FromMeta {
    /// A constructor that supposed to create default object for when there is only keyword with no arguments.
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Err(darling::Error::custom(
            "Can't be used as plain keyword, arguments required",
        ))
    }

    fn with_literals() -> bool {
        WITH_LITERALS
    }

    fn set_literals(self, literals: &Vec<Lit>) -> darling::Result<Self> {
        if WITH_LITERALS {
            Err(darling::Error::custom(format!(
                "{} must implement set_literals() method",
                std::any::type_name_of_val(&self)
            ))
            .with_span(&literals[0]))
        }
        else {
            self.no_literals(literals)
        }
    }

    fn no_literals(&self, literals: &Vec<Lit>) -> darling::Result<Self> {
        Err(darling::Error::custom("Literal values are not supported here").with_span(&literals[0]))
    }
}

#[derive(Debug, Clone, Getters)]
pub struct FXNestingAttr<T, const WITH_LITERALS: bool = true>
where
    T: FromNestAttr<WITH_LITERALS>,
{
    inner: T,
    orig:  Option<Meta>,
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> FXNestingAttr<T, WITH_LITERALS> {
    #[inline]
    pub fn new(inner: T, orig: Option<Meta>) -> Self {
        Self { inner, orig }
    }

    fn extract_literals(item: &Meta) -> darling::Result<Self> {
        let Meta::List(list) = item
        else {
            return Err(darling::Error::custom(format!(
                "internal problem: didn't expect {} here",
                std::any::type_name_of_val(&item)
            ))
            .with_span(item));
        };
        let nlist = NestedMeta::parse_meta_list(list.tokens.clone())?;
        let mut non_lit: Vec<NestedMeta> = vec![];
        let mut literals: Vec<Lit> = vec![];

        for item in nlist {
            match item {
                NestedMeta::Meta(ref meta) => {
                    non_lit.push(NestedMeta::Meta(meta.clone()));
                }
                NestedMeta::Lit(ref lit) => {
                    literals.push(lit.clone());
                }
            }
        }

        let mut fattr = T::from_list(&non_lit)?;
        if literals.len() > 0 {
            fattr = fattr.set_literals(&literals)?;
        }

        Ok(Self {
            inner: fattr,
            orig:  Some(item.clone()),
        })
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> FXOrig<syn::Meta> for FXNestingAttr<T, WITH_LITERALS> {
    #[inline]
    fn orig(&self) -> Option<&syn::Meta> {
        self.orig.as_ref()
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> FromMeta for FXNestingAttr<T, WITH_LITERALS> {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(match &item {
            Meta::List(_) => {
                if T::with_literals() {
                    Self::extract_literals(item)?
                }
                else {
                    Self {
                        inner: T::from_meta(item)?,
                        orig:  Some(item.clone()),
                    }
                }
            }
            Meta::Path(ref path) => Self {
                inner: T::for_keyword(path)?,
                orig:  Some(item.clone()),
            },
            _ => Self {
                inner: T::from_meta(item)?,
                orig:  Some(item.clone()),
            },
        })
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> Deref for FXNestingAttr<T, WITH_LITERALS> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> DerefMut for FXNestingAttr<T, WITH_LITERALS> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> AsRef<T> for FXNestingAttr<T, WITH_LITERALS>
where
    <FXNestingAttr<T, WITH_LITERALS> as Deref>::Target: AsRef<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> AsMut<T> for FXNestingAttr<T, WITH_LITERALS> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> ToTokens for FXNestingAttr<T, WITH_LITERALS> {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(ref orig) = self.orig {
            orig.to_tokens(tokens);
        }
    }
}

impl<T, U, const WITH_LITERALS: bool> FXFrom<U> for FXNestingAttr<T, WITH_LITERALS>
where
    T: FXFrom<U> + FromNestAttr<WITH_LITERALS>,
{
    #[inline]
    fn fx_from(value: U) -> Self {
        Self {
            inner: T::fx_from(value),
            orig:  None,
        }
    }
}

impl<T, U, const WITH_LITERALS: bool> FXFrom<U> for Option<FXNestingAttr<T, WITH_LITERALS>>
where
    T: FXFrom<U> + FromNestAttr<WITH_LITERALS>,
{
    #[inline]
    fn fx_from(value: U) -> Self {
        Some(FXNestingAttr {
            inner: T::fx_from(value),
            orig:  None,
        })
    }
}

impl<T, const WITH_LITERALS: bool> FXTriggerHelper for FXNestingAttr<T, WITH_LITERALS>
where
    T: FXTriggerHelper + FromNestAttr<WITH_LITERALS>,
{
    #[inline(always)]
    fn is_true(&self) -> bool {
        self.inner.is_true()
    }
}
