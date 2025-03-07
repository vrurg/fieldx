//! Argument nesting support module.
//!
//! In `fieldx` documentation these are named as _list_ or _function_ type arguments. For example, a typical nested
//! argument is `get` which can be used as a plain keyword, as `get(copy)`, or `get("getter", clone)`. This kind of
//! arguments require some additional amount of work to be implemented and this module is aimed as simplifying the task.
//!
//! Nesting is implemented as a two-stage construct. Parsing of its syntax structure is basically the same for all kinds
//! of arguments, it's only their sub-arguments that require specialization. Thus, the idea is to have a nesting parser
//! in a form of a container class which wraps objects implementing [`FromNestAttr`] trait.
//!
//! Subarguments are categorized as "literals" and "non-literals". In the `get` example above `"getter"` is a literal.
//! See [`syn::Lit`] type for more details.
//!
//! Also, argument is allowed to be used in a form of plain keyword with no subarguments, like `get`.

use crate::{with_origin::FXOrig, FXFrom, FXProp, FXTriggerHelper};
use darling::{ast::NestedMeta, FromMeta};
use getset::Getters;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::ops::{Deref, DerefMut};
use syn::{Lit, Meta};

/// This trait is for types that would like to be able to support nesting. See [module
/// documentation](crate::nesting_attr) for description.
///
/// Setting type parameter WITH_LITERALS to `false` disables literal subarguments.
pub trait FromNestAttr<const WITH_LITERALS: bool = true>: FromMeta {
    /// Constructor that supposed to create default object for when there is only keyword with no subarguments.
    /// Default behavior is to error out.
    fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
        Err(darling::Error::custom("Can't be used as plain keyword, arguments required").with_span(path))
    }

    /// Wether literal subarguments are allowed. Defaults to the `WITH_LITERALS` type parameter.
    fn with_literals() -> bool {
        WITH_LITERALS
    }

    /// Trait implementation must always override this method if `WITH_LITERALS` parameter is `true`. If it is `false`
    /// then the method produces a standard error "literals are not supported".
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

    /// Produce standard error when literal subarguments are used with argument not supporting them.
    fn no_literals(&self, literals: &Vec<Lit>) -> darling::Result<Self> {
        Err(darling::Error::custom("Literal values are not supported here").with_span(&literals[0]))
    }
}

/// The nesting container.
///
/// `WITH_LITERALS` parameter enables or disables support for literal subarguments.
#[derive(Debug, Clone, Getters)]
pub struct FXNestingAttr<T, const WITH_LITERALS: bool = true>
where
    T: FromNestAttr<WITH_LITERALS>,
{
    inner: T,
    orig:  Option<Meta>,
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> FXNestingAttr<T, WITH_LITERALS> {
    /// `orig` parameter is the syntax object from which we're constructing argument instance
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
    fn is_true(&self) -> FXProp<bool> {
        self.inner.is_true()
    }
}
