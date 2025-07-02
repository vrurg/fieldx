use std::ops::Deref;

use darling::util::Flag;
use proc_macro2::Span;

use crate::FXSetState;
use crate::FXSpaned;

/// The purpose of properties in FieldX is to provide a value with its source represented by a span.
///
/// The intended use is to allow tracking the origin of a computed value no matter how many steps it has gone through.
/// For example, a boolean that indicates whether or not we need to implement the `Default` trait for a struct can be
/// sourced from:
///
/// 1. The explicit `default` argument at the struct level.
/// 2. An explicit `default` argument of a field.
/// 3. An explicit `default` sub-argument of a field `serde` argument.
/// 4. A lazy attribute of a sync-mode field.
/// 5. Disabled deserialization of a field while struct deserialization is enabled.
///
/// As you can see, knowing what span we need to attach to the implementation of the `Default` trait could be quite
/// a non-trivial task! With `FXProp`, we can, say, just return a copy of a field `default` argument if it is set – and that's it,
/// the span is preserved automatically. In slightly more complex cases, we may need to create a new `FXProp` but still
/// only take the span from the original value we used for the new property.
#[derive(Debug)]
pub struct FXProp<T> {
    value: T,
    span:  Option<Span>,
}

impl<T> FXProp<T> {
    /// The constructor for `FXProp`.
    pub const fn new(value: T, span: Option<Span>) -> Self {
        Self { value, span }
    }

    /// Accessor for the value of the property.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Accessor for the orignal span of the property.
    pub fn orig_span(&self) -> Option<Span> {
        self.span
    }

    /// Return the original span of the property if it is set. Otherwise, return the default `Span::call_site()`.
    pub fn final_span(&self) -> Span {
        self.span.unwrap_or_else(Span::call_site)
    }

    /// Set span if it is not already set and do nothing otherwise.
    pub fn respan(mut self, span: Option<Span>) -> Self {
        if span.is_some() && self.span.is_none() {
            self.span = span;
        }
        self
    }
}

impl FXProp<bool> {
    /// Wraps the property into `Some` if it is `true`, otherwise returns `None`.
    pub fn true_or_none(&self) -> Option<&Self> {
        if self.value {
            Some(self)
        }
        else {
            None
        }
    }
}

impl<T> FXProp<T>
where
    T: FXSetState,
{
    /// Return the property itself if its value [`is_set`][FXSetState::is_set], otherwise returns the `other` property.
    pub fn or<'a>(&'a self, other: &'a FXProp<T>) -> &'a Self {
        if *self.is_set() {
            self
        }
        else {
            other
        }
    }
}

impl<T> FXSpaned for FXProp<T> {
    fn fx_span(&self) -> Span {
        self.final_span()
    }
}

impl<T> Default for FXProp<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            value: Default::default(),
            span:  None,
        }
    }
}

impl<T> Deref for FXProp<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Clone for FXProp<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            span:  self.span,
        }
    }
}

impl<T> Copy for FXProp<T> where T: Copy {}

impl<T> From<FXProp<T>> for bool
where
    T: FXSetState,
{
    fn from(value: FXProp<T>) -> Self {
        *value.is_set()
    }
}

impl From<FXProp<bool>> for bool {
    fn from(value: FXProp<bool>) -> Self {
        value.value
    }
}

impl From<&FXProp<bool>> for bool {
    fn from(value: &FXProp<bool>) -> Self {
        value.value
    }
}

impl From<Flag> for FXProp<bool> {
    fn from(value: Flag) -> Self {
        if value.is_present() {
            Self::new(true, Some(value.span()))
        }
        else {
            Self::new(false, None)
        }
    }
}

impl From<&Flag> for FXProp<bool> {
    fn from(value: &Flag) -> Self {
        Self::new(value.is_present(), Some(value.span()))
    }
}

macro_rules! from_lit_val {
    ( $($from_lit:path, $from_type:ty => $ty:ty);+ $(;)? ) => {
        $(from_lit_val!(@ $from_lit, $from_type => $ty);)+
    };
    (@ $from_lit:path, $from_type:ty => $ty:ty) => {
        impl From<$from_type> for FXProp<$ty> {
            fn from(value: $from_type) -> Self {
                FXProp::new(value.value(), Some(value.span()))
            }
        }

        impl From<&$from_type> for FXProp<$ty> {
            fn from(value: &$from_type) -> Self {
                FXProp::new(value.value(), Some(value.span()))
            }
        }

        impl TryFrom<::syn::Lit> for FXProp<$ty> {
            type Error = darling::Error;

            fn try_from(value: ::syn::Lit) -> Result<Self, Self::Error> {
                match value {
                    $from_lit(lit) => Ok(FXProp::new(lit.value(), Some(lit.span()))),
                    _ => Err(
                        darling::Error::custom(format!("The value must be a {}", stringify!($ty))).with_span(&value),
                    ),
                }
            }
        }

        impl TryFrom<&::syn::Lit> for FXProp<$ty> {
            type Error = darling::Error;

            fn try_from(value: &::syn::Lit) -> Result<Self, Self::Error> {
                match value {
                    $from_lit(lit) => Ok(FXProp::new(lit.value(), Some(lit.span()))),
                    _ => Err(
                        darling::Error::custom(format!("The value must be a {}", stringify!($ty))).with_span(&value),
                    ),
                }
            }
        }
    };
}

macro_rules! from_lit_num {
    ( $($from_lit:path, $from_type:ty => $ty:ty);+ $(;)? ) => {
        $(from_lit_num!(@ $from_lit, $from_type => $ty);)+
    };
    (@ $from_lit:path, $from_type:ty => $ty:ty ) => {
        impl TryFrom<$from_type> for FXProp<$ty> {
            type Error = darling::Error;

            fn try_from(value: $from_type) -> Result<Self, darling::Error> {
                Ok(FXProp::new(value.base10_parse()?, Some(value.span())))
            }
        }

        impl TryFrom<&$from_type> for FXProp<$ty> {
            type Error = darling::Error;

            fn try_from(value: &$from_type) -> Result<Self, darling::Error> {
                Ok(FXProp::new(value.base10_parse()?, Some(value.span())))
            }
        }

        impl TryFrom<::syn::Lit> for FXProp<$ty> {
            type Error = darling::Error;

            fn try_from(value: ::syn::Lit) -> Result<Self, Self::Error> {
                match value {
                    $from_lit(lit) => Ok(FXProp::new(lit.base10_parse()?, Some(lit.span()))),
                    _ => Err(
                        darling::Error::custom(format!("The value must be a {}", stringify!($ty))).with_span(&value),
                    ),
                }
            }
        }

        impl TryFrom<&::syn::Lit> for FXProp<$ty> {
            type Error = darling::Error;

            fn try_from(value: &::syn::Lit) -> Result<Self, Self::Error> {
                match value {
                    $from_lit(lit) => Ok(FXProp::new(lit.base10_parse()?, Some(lit.span()))),
                    _ => Err(
                        darling::Error::custom(format!("The value must be a {}", stringify!($ty))).with_span(&value),
                    ),
                }
            }
        }
    };
}

from_lit_val! {
    syn::Lit::Bool,     syn::LitBool    => bool;
    syn::Lit::ByteStr,  syn::LitByteStr => Vec<u8>;
    syn::Lit::Char,     syn::LitChar    => char;
    syn::Lit::Str,      syn::LitStr     => String;
}

from_lit_num! {
    syn::Lit::Float,    syn::LitFloat => f32;
    syn::Lit::Float,    syn::LitFloat => f64;
    syn::Lit::Int,      syn::LitInt   => i16;
    syn::Lit::Int,      syn::LitInt   => i32;
    syn::Lit::Int,      syn::LitInt   => i64;
    syn::Lit::Int,      syn::LitInt   => i8;
    syn::Lit::Int,      syn::LitInt   => u16;
    syn::Lit::Int,      syn::LitInt   => u32;
    syn::Lit::Int,      syn::LitInt   => u64;
    syn::Lit::Int,      syn::LitInt   => u8;
}

impl From<bool> for FXProp<bool> {
    fn from(value: bool) -> Self {
        Self::new(value, None)
    }
}

impl<T> From<FXProp<T>> for FXProp<bool>
where
    T: FXSetState,
{
    fn from(value: FXProp<T>) -> Self {
        value.is_set()
    }
}

/// Implement logical chaining of values. Though not hard limited to use with [`FXProp`] but the associated types are
/// generally expected to be an `FXProp` throught FieldX codebase.
pub trait FXPropBool {
    /// The return type of the `or` method.
    type Or;
    /// The return type of the `not` method.
    type Not;

    /// Must return `self` if it is true-ish, otherwise return `other`.
    fn or(self, other: Self) -> Self::Or;
    /// Must return a negated version of `self`.
    fn not(self) -> Self::Not;
}

impl<'a> FXPropBool for &'a FXProp<bool> {
    type Not = FXProp<bool>;
    type Or = &'a FXProp<bool>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        if self.value {
            self
        }
        else {
            other
        }
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        FXProp::new(!self.value, self.span)
    }
}

impl FXPropBool for FXProp<bool> {
    type Not = FXProp<bool>;
    type Or = FXProp<bool>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        if self.value {
            self
        }
        else {
            other
        }
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        FXProp::new(!self.value, self.span)
    }
}

impl<'a> FXPropBool for Option<&'a FXProp<bool>> {
    type Not = Option<FXProp<bool>>;
    type Or = Option<&'a FXProp<bool>>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        self.and_then(|s| s.true_or_none())
            .or_else(|| other.and_then(|o| o.true_or_none()))
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        self.map(|f| FXProp::new(!f.value, f.span))
    }
}

impl FXPropBool for Option<FXProp<bool>> {
    type Not = Option<FXProp<bool>>;
    type Or = Option<FXProp<bool>>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        self.and_then(|s| s.true_or_none().copied())
            .or_else(|| other.and_then(|o| o.true_or_none().copied()))
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        self.map(|f| FXProp::new(!f.value, f.span))
    }
}

impl<'a> FXPropBool for &'a Option<FXProp<bool>> {
    type Not = Option<FXProp<bool>>;
    type Or = Option<&'a FXProp<bool>>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        self.as_ref()
            .and_then(|s| s.true_or_none())
            .or_else(|| other.as_ref().and_then(|o| o.true_or_none()))
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        self.map(|f| FXProp::new(!f.value, f.span))
    }
}

impl FXSetState for FXProp<bool> {
    fn is_set(&self) -> FXProp<bool> {
        *self
    }
}

impl<T> FXSetState for FXProp<T>
where
    T: FXSetState,
{
    fn is_set(&self) -> FXProp<bool> {
        self.value.is_set().respan(self.span)
    }
}
