//! Support for types that are not supported by [`darling`] but implement [`syn::parse::Parse`]
use super::{FXFrom, FromNestAttr};
use darling::{ast::NestedMeta, FromMeta};
use quote::ToTokens;
use std::{borrow::Borrow, fmt::Debug, marker::PhantomData, ops::Deref};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Meta};

/// Argument that takes exactly one syntax element.
///
/// `AS_KEYWORD` parameter enables/disables use of the argument as a plain keyword with no subargument.
///
/// For example:
///
/// ```ignore
///     foo: FXNestingAttr<FXSynValueArg<syn::Expr>>,
/// ```
///
/// Allows the `foo` argument to take whatever Rust expression is allowed: `foo(|v| v.method())`,
/// `foo(if true { println!("OK!") })`, etc.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct FXSynValueArg<T, const AS_KEYWORD: bool = false> {
    value: Option<T>,
}

impl<T> FXSynValueArg<T, false> {
    /// Accessor to the actual syntax object.
    pub fn value(&self) -> &T {
        self.value.as_ref().unwrap()
    }
}

impl<T> FXSynValueArg<T, true> {
    /// Accessor to the actual syntax object if set.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }
}

impl<T, const AS_KEYWORD: bool> FromMeta for FXSynValueArg<T, AS_KEYWORD>
where
    T: Parse,
{
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(Self {
            value: Some(match item {
                Meta::List(list) => syn::parse2(list.tokens.clone())?,
                _ => return Err(darling::Error::unsupported_format("argument is expected")),
            }),
        })
    }

    fn from_list(_items: &[NestedMeta]) -> darling::Result<Self> {
        Err(darling::Error::unsupported_format("NYI"))
    }
}

impl<T, const AS_KEYWORD: bool> From<T> for FXSynValueArg<T, AS_KEYWORD>
where
    T: FromMeta,
{
    fn from(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T, const AS_KEYWORD: bool> FXFrom<T> for FXSynValueArg<T, AS_KEYWORD>
where
    T: FromMeta,
{
    fn fx_from(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T> FromNestAttr<false> for FXSynValueArg<T, false> where T: Parse {}

impl<T> FromNestAttr<false> for FXSynValueArg<T, true>
where
    T: Parse,
{
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self { value: None })
    }
}

impl<T> Deref for FXSynValueArg<T, false> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.as_ref().unwrap()
    }
}

impl<T> Deref for FXSynValueArg<T, true> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> AsRef<T> for FXSynValueArg<T, false> {
    fn as_ref(&self) -> &T {
        self.value.as_ref().unwrap()
    }
}

impl<T> AsRef<Option<T>> for FXSynValueArg<T, true> {
    fn as_ref(&self) -> &Option<T> {
        &self.value
    }
}

impl<T> Borrow<T> for FXSynValueArg<T, false> {
    fn borrow(&self) -> &T {
        self.value.as_ref().unwrap()
    }
}

impl<T> Borrow<Option<T>> for FXSynValueArg<T, true> {
    fn borrow(&self) -> &Option<T> {
        &self.value
    }
}

/// Argument that takes 2 to 10 syntax elements.
///
/// For example:
///
/// ```ignore
///     foo: FXNestingAttr<FXSynTupleArg<(syn::Path, syn::PatRange)>>,
/// ```
///
/// `foo` can be used as `foo(std::sync::Arc, 1..=42)`.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct FXSynTupleArg<T> {
    value: T,
}

impl<T> FXSynTupleArg<T> {
    /// Accessor to the actual tuple of syntax objects.
    pub fn value(&self) -> &T {
        &self.value
    }
}

macro_rules! from_tuple {
    ( $( ( $( $ty:ident ),+ ) ),+ $(,)* ) => {
        $(
            impl< $( $ty, )+ > FromNestAttr<false> for FXSynTupleArg<( $( $ty ),+ )>
            where $( $ty: syn::parse::Parse ),+
            {}

            impl< $( $ty, )+ > FromMeta for FXSynTupleArg<( $( $ty ),+ )>
            where $( $ty: syn::parse::Parse ),+
            {
                fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
                    let expected = from_tuple!(@count $($ty),+);
                    if items.len() > expected {
                        return Err(darling::Error::too_many_items(expected));
                    }
                    if items.len() < expected {
                        return Err(darling::Error::too_few_items(expected));
                    }
                    let mut iter = items.into_iter();
                    Ok(Self {
                        value: ( $( syn::parse2::<$ty>(iter.next().to_token_stream())? ),+ )
                    })
                }
            }
        )+
    };

    (@count $head:ident, $( $ty:ident ),+ ) => {
        1 + from_tuple!(@count $( $ty ),+ )
    };
    (@count $ty:ident ) => { 1 };
}

from_tuple! {
    (T1, T2),
    (T1, T2, T3),
    (T1, T2, T3, T4),
    (T1, T2, T3, T4, T5),
    (T1, T2, T3, T4, T5, T6),
    (T1, T2, T3, T4, T5, T6, T7),
    (T1, T2, T3, T4, T5, T6, T7, T8),
    (T1, T2, T3, T4, T5, T6, T7, T8, T9),
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
}

/// Argument that takes a list of syntax elements of the same type.
///
/// Type parameters:
///
/// - `T` – the actual type of the syntax object
/// - `S` – separator token (see [`syn::Token`] macro)
///
/// For example:
///
/// ```ignore
///     foo: FXPunctuated<syn::PatType, Token![,]>,
/// ```
///
/// Usage: `foo(f1: i32, f2: String)`.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct FXPunctuated<T, S, const MIN: i32 = -1, const MAX: i32 = -1>
where
    T: Debug + Spanned + ToTokens + Parse,
    S: Debug + Spanned + ToTokens + Parse,
{
    items: Vec<T>,
    _p:    PhantomData<S>,
}

impl<T, S, const MIN: i32, const MAX: i32> FXPunctuated<T, S, MIN, MAX>
where
    T: Debug + Spanned + ToTokens + Parse,
    S: Debug + Spanned + ToTokens + Parse,
{
    /// Accessor for the syntax objects list.
    pub fn items(&self) -> &Vec<T> {
        &self.items
    }
}

impl<T, S, const MIN: i32, const MAX: i32> syn::parse::Parse for FXPunctuated<T, S, MIN, MAX>
where
    T: Debug + Spanned + ToTokens + Parse,
    S: Debug + Spanned + ToTokens + Parse,
{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let result = Punctuated::<T, S>::parse_terminated(input)?;
        let count = result.len();
        if MIN >= 0 && count < MIN as usize {
            return Err(darling::Error::too_few_items(MIN as usize)
                .with_span(&result.span())
                .into());
        }
        if MAX >= 0 && count > MAX as usize {
            return Err(darling::Error::too_many_items(MAX as usize)
                .with_span(&result.span())
                .into());
        }

        Ok(Self {
            items: result.into_iter().collect(),
            _p:    PhantomData::default(),
        })
    }
}

impl<T, S, const MIN: i32, const MAX: i32> FromMeta for FXPunctuated<T, S, MIN, MAX>
where
    T: Spanned + ToTokens + Parse + Debug,
    S: Spanned + ToTokens + Parse + Debug,
{
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(match item {
            Meta::List(ref list) => syn::parse2(list.tokens.clone())?,
            _ => syn::parse2(item.to_token_stream())?,
        })
    }
}

impl<T, S, const MIN: i32, const MAX: i32> Deref for FXPunctuated<T, S, MIN, MAX>
where
    T: Spanned + ToTokens + Parse + Debug,
    S: Spanned + ToTokens + Parse + Debug,
{
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T, S, const MIN: i32, const MAX: i32> AsRef<Vec<T>> for FXPunctuated<T, S, MIN, MAX>
where
    T: Spanned + ToTokens + Parse + Debug,
    S: Spanned + ToTokens + Parse + Debug,
{
    fn as_ref(&self) -> &Vec<T> {
        &self.items
    }
}

impl<T, S, const MIN: i32, const MAX: i32> Borrow<Vec<T>> for FXPunctuated<T, S, MIN, MAX>
where
    T: Spanned + ToTokens + Parse + Debug,
    S: Spanned + ToTokens + Parse + Debug,
{
    fn borrow(&self) -> &Vec<T> {
        &self.items
    }
}
