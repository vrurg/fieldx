//! Support for types that are not supported by [`darling`] but implement [`syn::parse::Parse`]
use crate::traits::FXSetState;
use crate::FXProp;
use crate::FXTryFrom;

use super::FXFrom;
use super::FromNestAttr;

use darling::FromMeta;
use paste::paste;
use quote::quote;
use quote::ToTokens;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Meta;

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

impl<T> FXSetState for FXSynValueArg<T, false> {
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(self.value.is_some(), None)
    }
}

// If can be used as a keyword then it's always set because value becomes just a helpful addition to the main purpose
// of the argument.
impl<T> FXSetState for FXSynValueArg<T, true> {
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(true, None)
    }
}

impl<T, const AS_KEYWORD: bool> ToTokens for FXSynValueArg<T, AS_KEYWORD>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if let Some(value) = &self.value {
            value.to_tokens(tokens);
        }
    }
}

impl<T, const AS_KEYWORD: bool> FromMeta for FXSynValueArg<T, AS_KEYWORD>
where
    T: Parse,
{
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Ok(Self {
            value: match item {
                Meta::List(list) => {
                    if list.tokens.is_empty() && AS_KEYWORD {
                        None
                    }
                    else {
                        Some(syn::parse2(list.tokens.clone())?)
                    }
                }
                _ => {
                    return Err(darling::Error::unsupported_format(
                        "must be a function-call-like argument",
                    ))
                }
            },
        })
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

impl<T, U, const AS_KEYWORD: bool> From<FXSynValueArg<T, AS_KEYWORD>> for Option<FXProp<U>>
where
    FXProp<U>: From<T>,
    T: Spanned,
{
    fn from(value: FXSynValueArg<T, AS_KEYWORD>) -> Self {
        value.value.map(|v| v.into())
    }
}

impl<T, U, const AS_KEYWORD: bool> From<&FXSynValueArg<T, AS_KEYWORD>> for Option<FXProp<U>>
where
    FXProp<U>: for<'a> From<&'a T>,
    T: Spanned,
{
    fn from(value: &FXSynValueArg<T, AS_KEYWORD>) -> Self {
        value.value.as_ref().map(|v| v.into())
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

impl<T, const AS_KEYWORD: bool> FXTryFrom<T> for FXSynValueArg<T, AS_KEYWORD>
where
    T: Parse,
{
    type Error = darling::Error;

    fn fx_try_from(value: T) -> Result<Self, Self::Error> {
        Ok(Self { value: Some(value) })
    }
}

impl<T, const AS_KEYWORD: bool> FXTryFrom<Option<T>> for FXSynValueArg<T, AS_KEYWORD>
where
    T: Parse,
{
    type Error = darling::Error;

    fn fx_try_from(value: Option<T>) -> Result<Self, Self::Error> {
        Ok(Self { value })
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

impl<T> FXSetState for FXSynTupleArg<T> {
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(true, None)
    }
}

macro_rules! from_tuple {
    ( $( ( $ty1:ident, $( $ty:ident ),+ ) ),+ $(,)* ) => {
        $(
            impl< $ty1, $( $ty, )+ > FromNestAttr<false> for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            where $ty1: syn::parse::Parse, $( $ty: syn::parse::Parse ),+
            {}

            impl< $ty1, $( $ty, )+ > FromMeta for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            where $ty1: syn::parse::Parse, $( $ty: syn::parse::Parse ),+
            {
                fn from_meta(item: &Meta) -> darling::Result<Self> {
                    Ok(match item {
                        Meta::List(ref list) => syn::parse2(list.tokens.clone())?,
                        _ => Err(darling::Error::unsupported_format("unsupported format"))?,
                    })
                }
            }

            impl< $ty1, $( $ty, )+ > Parse for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            where $ty1: syn::parse::Parse, $( $ty: syn::parse::Parse ),+
            {
                fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {

                    Ok(Self { value: (
                        input.parse::<$ty1>()?,
                        $( {
                            input.parse::<syn::Token![,]>()?;
                            input.parse::<$ty>()?
                        }, )+
                    ) })
                }
            }

            impl< $ty1, $( $ty, )+ > ToTokens for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            where $ty1: ToTokens, $( $ty: ToTokens ),+
            {
                fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                    paste! {
                        let ( [<v_ $ty1:lower>], $( [<v_ $ty:lower >] ),+ ) = &self.value;
                        tokens.extend(quote! {
                            #[<v_ $ty1:lower>] $(, #[<v_ $ty:lower>])*
                        });
                    }
                }
            }

            impl< $ty1, $( $ty, )+ > Deref for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            {
                type Target = ( $ty1, $( $ty ),+ );

                fn deref(&self) -> &Self::Target {
                    &self.value
                }
            }

            impl< $ty1, $( $ty, )+ > AsRef<( $ty1, $( $ty ),+ )> for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            {
                fn as_ref(&self) -> &( $ty1, $( $ty ),+ ) {
                    &self.value
                }
            }

            impl< $ty1, $( $ty, )+ > Borrow<( $ty1, $( $ty ),+ )> for FXSynTupleArg<( $ty1, $( $ty ),+ )>
            {
                fn borrow(&self) -> &( $ty1, $( $ty ),+ ) {
                    &self.value
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
    items: syn::punctuated::Punctuated<T, S>,
    _p:    PhantomData<S>,
}

impl<T, S, const MIN: i32, const MAX: i32> FXPunctuated<T, S, MIN, MAX>
where
    T: Debug + Spanned + ToTokens + Parse,
    S: Debug + Spanned + ToTokens + Parse,
{
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T, S, const MIN: i32, const MAX: i32> FXSetState for FXPunctuated<T, S, MIN, MAX>
where
    T: Debug + Spanned + ToTokens + Parse,
    S: Debug + Spanned + ToTokens + Parse,
{
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(true, None)
    }
}

impl<T, S, const MIN: i32, const MAX: i32> ToTokens for FXPunctuated<T, S, MIN, MAX>
where
    T: Debug + Spanned + ToTokens + Parse,
    S: Debug + Spanned + ToTokens + Parse,
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.items.to_tokens(tokens);
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
            items: result.into_pairs().collect(),
            _p:    PhantomData,
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
    type Target = Punctuated<T, S>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T, S, const MIN: i32, const MAX: i32> AsRef<Punctuated<T, S>> for FXPunctuated<T, S, MIN, MAX>
where
    T: Spanned + ToTokens + Parse + Debug,
    S: Spanned + ToTokens + Parse + Debug,
{
    fn as_ref(&self) -> &Punctuated<T, S> {
        &self.items
    }
}

impl<T, S, const MIN: i32, const MAX: i32> Borrow<Punctuated<T, S>> for FXPunctuated<T, S, MIN, MAX>
where
    T: Spanned + ToTokens + Parse + Debug,
    S: Spanned + ToTokens + Parse + Debug,
{
    fn borrow(&self) -> &Punctuated<T, S> {
        &self.items
    }
}

// impl<T, S, const MIN: i32, const MAX: i32> ToTokens for FXPunctuated<T, S, MIN, MAX>
// where
//     T: Spanned + ToTokens + Parse + Debug,
//     S: Spanned + ToTokens + Parse + Debug + Default,
// {
//     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
//         for item in self.items.iter() {
//             item.to_tokens(tokens);
//             let sep = S { span: item.span() };
//             tokens.extend(sep.clone());
//         }
//     }
// }

#[cfg(test)]
mod test {
    use darling::ast;
    use darling::FromMeta;
    use quote::quote;
    use quote::ToTokens;

    #[test]
    fn tuple() {
        use super::FXSynTupleArg;
        use crate::FXSynTuple;

        let input = quote! {module::Foo, self.bar(), 42};

        let arg = syn::parse2::<FXSynTupleArg<(syn::Path, syn::Expr, syn::Lit)>>(input.clone()).unwrap();

        assert_eq!(arg.0.to_token_stream().to_string(), quote! {module::Foo}.to_string());
        assert_eq!(arg.1.to_token_stream().to_string(), quote! {self.bar()}.to_string());
        assert_eq!(arg.2.to_token_stream().to_string(), quote! {42}.to_string());

        if let Err(e) =
            syn::parse2::<FXSynTupleArg<(syn::Path, syn::Expr, syn::Lit)>>(quote! {module::Foo, self.bar(), v2})
        {
            assert!(e.to_string().contains("expected literal"));
        }
        else {
            panic!("Expected error");
        }

        assert_eq!(arg.to_token_stream().to_string(), input.to_string());

        let input = quote! {foo(MyType, MyType::fun("something"), 3.1415926)};
        let meta: ast::NestedMeta = syn::parse2::<ast::NestedMeta>(input.clone()).unwrap();
        let arg = FXSynTuple::<(syn::Path, syn::Expr, syn::Lit)>::from_nested_meta(&meta).unwrap();
        assert_eq!(arg.to_token_stream().to_string(), input.to_string());
        assert_eq!(arg.0.to_token_stream().to_string(), quote! {MyType}.to_string());
        assert_eq!(
            arg.1.to_token_stream().to_string(),
            quote! {MyType::fun("something")}.to_string()
        );
        assert_eq!(arg.2.to_token_stream().to_string(), quote! {3.1415926}.to_string());
    }
}

#[cfg(test)]
mod tests {
    use darling::FromMeta;
    use quote::quote;

    use crate::FXNestingAttr;

    use super::FXSynValueArg;

    #[test]
    fn no_args() {
        let input = quote! {foo()};
        let meta: syn::Meta = syn::parse2(input).unwrap();
        let nest: FXNestingAttr<FXSynValueArg<syn::Expr, true>, false> = FromMeta::from_meta(&meta).unwrap();

        assert!(nest.value().is_none());
    }
}
