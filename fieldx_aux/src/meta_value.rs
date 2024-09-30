use std::{borrow::Borrow, marker::PhantomData, ops::Deref};

use super::{FXFrom, FromNestAttr};
use darling::{ast::NestedMeta, FromMeta};
use quote::ToTokens;
use syn::{punctuated::Punctuated, Meta};

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct FXSynValueArg<T> {
    value: T,
}

impl<T> FXSynValueArg<T> {
    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T: syn::parse::Parse> FromMeta for FXSynValueArg<T> {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(Self {
            value: match item {
                Meta::List(list) => syn::parse2(list.tokens.clone())?,
                _ => return Err(darling::Error::unsupported_format("argument is expected")),
            },
        })
    }

    fn from_list(_items: &[NestedMeta]) -> darling::Result<Self> {
        Err(darling::Error::unsupported_format("NYI"))
    }
}

impl<T> From<T> for FXSynValueArg<T>
where
    T: FromMeta,
{
    fn from(value: T) -> Self {
        Self { value: value }
    }
}

impl<T> FXFrom<T> for FXSynValueArg<T>
where
    T: FromMeta,
{
    fn fx_from(value: T) -> Self {
        Self { value: value }
    }
}

impl<T: syn::parse::Parse> FromNestAttr<false> for FXSynValueArg<T> {}

impl<T> Deref for FXSynValueArg<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> AsRef<T> for FXSynValueArg<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T> Borrow<T> for FXSynValueArg<T> {
    fn borrow(&self) -> &T {
        &self.value
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct FXSynTupleArg<T> {
    value: T,
}

impl<T> FXSynTupleArg<T> {
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

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct FXPunctuated<T, S> {
    items: Vec<T>,
    _p:    PhantomData<S>,
}

impl<T, S> FXPunctuated<T, S> {
    pub fn items(&self) -> &Vec<T> {
        &self.items
    }
}

impl<T: syn::parse::Parse, S: syn::parse::Parse> syn::parse::Parse for FXPunctuated<T, S> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let result = Punctuated::<T, S>::parse_terminated(input)?;
        Ok(Self {
            items: result.into_iter().collect(),
            _p:    PhantomData::default(),
        })
    }
}

impl<T: syn::parse::Parse, S: syn::parse::Parse> FromMeta for FXPunctuated<T, S> {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(match item {
            Meta::List(ref list) => syn::parse2(list.tokens.clone())?,
            _ => syn::parse2(item.to_token_stream())?,
        })
    }
}

impl<T, S> Deref for FXPunctuated<T, S> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T, S> AsRef<Vec<T>> for FXPunctuated<T, S> {
    fn as_ref(&self) -> &Vec<T> {
        &self.items
    }
}

impl<T, S> Borrow<Vec<T>> for FXPunctuated<T, S> {
    fn borrow(&self) -> &Vec<T> {
        &self.items
    }
}
