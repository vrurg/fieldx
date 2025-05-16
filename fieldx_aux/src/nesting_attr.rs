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

use crate::with_origin::FXOrig;
use crate::FXInto;
use crate::FXProp;
use crate::FXSetState;
use crate::FXTryFrom;

use darling::ast::NestedMeta;
use darling::FromMeta;
use darling::ToTokens;
use getset::Getters;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use std::ops::Deref;
use std::ops::DerefMut;
use syn::spanned::Spanned;
use syn::Lit;
use syn::Meta;

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
    fn set_literals(self, literals: &[Lit]) -> darling::Result<Self> {
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
    fn no_literals(&self, literals: &[Lit]) -> darling::Result<Self> {
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
    /// The path can differ from that of the original meta. This can occur when an attribute or argument is assumed
    /// based on the value of another attribute or argument. For example, the plain use of `fieldx` implies the `lazy`,
    /// `clearer`, and `predicate` arguments, where the original meta is `fieldx` but the actual path is different.
    path:  Option<syn::Path>,
    orig:  Meta,
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> FXNestingAttr<T, WITH_LITERALS> {
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

        for nested in nlist {
            match nested {
                NestedMeta::Meta(ref meta) => {
                    non_lit.push(NestedMeta::Meta(meta.clone()));
                }
                NestedMeta::Lit(ref lit) => {
                    literals.push(lit.clone());
                }
            }
        }

        let mut fattr = T::from_list(&non_lit)?;
        if !literals.is_empty() {
            fattr = fattr.set_literals(&literals)?;
        }

        Ok(Self {
            inner: fattr,
            path:  None,
            orig:  item.clone(),
        })
    }

    fn from_path_tokens<P: Into<syn::Path>, TT: ToTokens>(path: P, tokens: TT) -> darling::Result<Self> {
        let path = path.into();
        let orig = syn::parse2::<syn::Meta>(quote_spanned! {tokens.span()=> #path(#tokens) })?;
        Self::from_meta(&orig)
    }

    pub fn from_tokens<TT: ToTokens>(toks: TT) -> darling::Result<Self> {
        let orig = syn::parse2::<syn::Meta>(quote_spanned! {toks.span()=> #toks })?;
        Self::from_meta(&orig)
    }
}

impl<T: FromNestAttr<WITH_LITERALS>, const WITH_LITERALS: bool> FXOrig<syn::Meta> for FXNestingAttr<T, WITH_LITERALS> {
    #[inline]
    fn orig(&self) -> Option<&syn::Meta> {
        Some(&self.orig)
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
                        path:  None,
                        orig:  item.clone(),
                    }
                }
            }
            Meta::Path(ref path) => Self {
                inner: T::for_keyword(path)?,
                path:  None,
                orig:  item.clone(),
            },
            _ => Self {
                inner: T::from_meta(item)?,
                path:  None,
                orig:  item.clone(),
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

impl<T: FromNestAttr<WITH_LITERALS> + ToTokens, const WITH_LITERALS: bool> ToTokens
    for FXNestingAttr<T, WITH_LITERALS>
{
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let inner_toks = self.inner.to_token_stream();
        let path = self.path.as_ref().unwrap_or_else(|| self.orig.path());
        tokens.extend(quote_spanned! {path.span()=> #path(#inner_toks) });
    }
}

impl<T, P, U, const WITH_LITERALS: bool> FXTryFrom<(P, U)> for FXNestingAttr<T, WITH_LITERALS>
where
    (P, U): FXInto<syn::Path>,
    U: ToTokens + Clone,
    T: FXTryFrom<U> + FromNestAttr<WITH_LITERALS>,
{
    type Error = darling::Error;

    #[inline]
    fn fx_try_from(value: (P, U)) -> darling::Result<Self> {
        let v1 = value.1.clone();
        Self::from_path_tokens(value.fx_into(), v1)
    }
}

impl<T, P, U, const WITH_LITERALS: bool> FXTryFrom<(P, U)> for Option<FXNestingAttr<T, WITH_LITERALS>>
where
    (P, U): FXInto<syn::Path>,
    U: ToTokens + Clone,
    T: FXTryFrom<U> + FromNestAttr<WITH_LITERALS>,
{
    type Error = darling::Error;

    #[inline]
    fn fx_try_from(value: (P, U)) -> darling::Result<Self> {
        let v1 = value.1.clone();
        Ok(Some(FXNestingAttr::from_path_tokens(value.fx_into(), v1)?))
    }
}

// impl<T, U, const WITH_LITERALS: bool> FXFrom<U> for Option<FXNestingAttr<T, WITH_LITERALS>>
// where
//     T: FXFrom<U> + FromNestAttr<WITH_LITERALS>,
// {
//     #[inline]
//     fn fx_from(value: U) -> Self {
//         Some(FXNestingAttr {
//             inner: T::fx_from(value),
//             orig:  None,
//         })
//     }
// }

// impl<T, U, const WITH_LITERALS: bool> FXTryFrom<U> for FXNestingAttr<T, WITH_LITERALS>
// where
//     T: FXTryFrom<U, Error = darling::Error> + FromNestAttr<WITH_LITERALS>,
// {
//     type Error = darling::Error;

//     #[inline]
//     fn fx_try_from(value: U) -> Result<Self, Self::Error> {
//         Ok(Self {
//             inner: T::fx_try_from(value)?,
//             orig:  None,
//         })
//     }
// }

// impl<T, U, const WITH_LITERALS: bool> FXTryFrom<U> for Option<FXNestingAttr<T, WITH_LITERALS>>
// where
//     T: FXTryFrom<U, Error = darling::Error> + FromNestAttr<WITH_LITERALS>,
// {
//     type Error = darling::Error;

//     #[inline]
//     fn fx_try_from(value: U) -> Result<Self, Self::Error> {
//         Ok(Some(FXNestingAttr {
//             inner: T::fx_try_from(value)?,
//             orig:  None,
//         }))
//     }
// }

impl<T, const WITH_LITERALS: bool> FXSetState for FXNestingAttr<T, WITH_LITERALS>
where
    T: FXSetState + FromNestAttr<WITH_LITERALS>,
{
    #[inline(always)]
    fn is_set(&self) -> FXProp<bool> {
        self.inner.is_set().respan(self.orig_span())
    }
}

impl<T, U, const WITH_LITERALS: bool> From<FXNestingAttr<T, WITH_LITERALS>> for Option<FXProp<U>>
where
    Option<FXProp<U>>: From<T>,
    T: FromNestAttr<WITH_LITERALS>,
{
    #[inline(always)]
    fn from(value: FXNestingAttr<T, WITH_LITERALS>) -> Self {
        let orig_span = value.orig_span();
        let p: Self = value.inner.into();
        p.map(|p| p.respan(orig_span))
    }
}

impl<T, U, const WITH_LITERALS: bool> From<&FXNestingAttr<T, WITH_LITERALS>> for Option<FXProp<U>>
where
    Option<FXProp<U>>: for<'a> From<&'a T>,
    T: FromNestAttr<WITH_LITERALS>,
{
    #[inline(always)]
    fn from(value: &FXNestingAttr<T, WITH_LITERALS>) -> Self {
        let p: Self = (&value.inner).into();
        p.map(|p| p.respan(value.orig_span()))
    }
}

// impl<T, const WITH_LITERALS: bool> FXSetState for &FXNestingAttr<T, WITH_LITERALS>
// where
//     T: FXSetState + FromNestAttr<WITH_LITERALS>,
// {
//     #[inline(always)]
//     fn is_set(&self) -> FXProp<bool> {
//         self.inner.is_set().respan(self.orig_span())
//     }
// }
