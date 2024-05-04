use super::FXOrig;
use darling::{ast::NestedMeta, FromMeta};
use getset::Getters;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::ops::{Deref, DerefMut};
use syn::{Lit, Meta};

pub(crate) trait FromNestAttr: FromMeta {
    /// A constructor that supposed to create default object for when there is only keyword with no arguments.
    fn for_keyword() -> darling::Result<Self>;
    fn set_literals(self, literals: &Vec<Lit>) -> darling::Result<Self>;

    fn no_literals(&self, literals: &Vec<Lit>) -> darling::Result<()> {
        Err(darling::Error::custom("Literal values are not supported here").with_span(&literals[0]))
    }
}

#[derive(Debug, Clone, Getters)]
pub(crate) struct FXNestingAttr<T: FromNestAttr> {
    inner: T,
    orig:  Option<Meta>,
}

impl<T: FromNestAttr> FXOrig<syn::Meta> for FXNestingAttr<T> {
    fn orig(&self) -> Option<&syn::Meta> {
        self.orig.as_ref()
    }
}

impl<T: FromNestAttr> FXNestingAttr<T> {
    pub(crate) fn new(inner: T, orig: Option<Meta>) -> Self {
        Self { inner, orig }
    }

    pub(crate) fn set_orig(mut self, orig: Meta) -> Self {
        self.orig = Some(orig);
        self
    }
}

impl<T: FromNestAttr> FromMeta for FXNestingAttr<T> {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        match &item {
            Meta::List(ref list) => {
                let nlist = NestedMeta::parse_meta_list(list.tokens.clone())?;
                Ok(Self::from_list(&nlist)?.set_orig(item.clone()))
            }
            Meta::Path(ref _path) => Ok(Self {
                inner: T::for_keyword()?,
                orig:  Some(item.clone()),
            }),
            _ => Err(darling::Error::custom("Unsupported argument format").with_span(item)),
        }
    }

    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let mut non_lit: Vec<NestedMeta> = vec![];
        let mut literals: Vec<Lit> = vec![];

        for item in items {
            match item {
                NestedMeta::Meta(ref meta) => {
                    non_lit.push(NestedMeta::Meta(meta.clone()));
                }
                NestedMeta::Lit(ref lit) => {
                    literals.push(lit.clone());
                }
            }
        }

        let fattr = T::from_list(&non_lit)?;
        Ok(Self {
            inner: if literals.len() > 0 {
                fattr.set_literals(&literals)?
            }
            else {
                fattr
            },
            orig:  None,
        })
    }
}

impl<T: FromNestAttr> Deref for FXNestingAttr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: FromNestAttr> DerefMut for FXNestingAttr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: FromNestAttr> AsRef<T> for FXNestingAttr<T>
where
    <FXNestingAttr<T> as Deref>::Target: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: FromNestAttr> AsMut<T> for FXNestingAttr<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: FromNestAttr> ToTokens for FXNestingAttr<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(ref orig) = self.orig {
            orig.to_tokens(tokens);
        }
    }
}
