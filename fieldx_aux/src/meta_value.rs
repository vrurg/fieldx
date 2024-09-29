use super::{FXFrom, FromNestAttr};
use darling::{ast::NestedMeta, FromMeta};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, parse_quote, punctuated::Punctuated, Meta, Token};

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

    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
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
