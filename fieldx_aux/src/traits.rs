use proc_macro2::Span;
use quote::format_ident;
use syn::spanned::Spanned;

use crate::FXAttributes;
use crate::FXDoc;
use crate::FXProp;

/// Where it is not possible to use the standard `From`/`Into` traits due to conflicting implementations this crate is
/// using `FXFrom`/`FXInto` instead.
pub trait FXFrom<T> {
    fn fx_from(value: T) -> Self;
}

/// A common way to create a `syn::Path` from a string literal using an [`FXSpaned`] value.
/// Helper for [`ToTokens`](https://docs.rs/quote/latest/quote/trait.ToTokens.html) implementations.
impl<S> FXFrom<(&str, S)> for syn::Path
where
    S: FXSpaned,
{
    #[inline]
    fn fx_from(value: (&str, S)) -> Self {
        format_ident!("{}", value.0, span = value.1.fx_span()).into()
    }
}

/// The counterpart of `FXFrom`.
pub trait FXInto<T> {
    fn fx_into(self) -> T;
}

impl<T, U> FXInto<U> for T
where
    U: FXFrom<T>,
{
    #[inline]
    fn fx_into(self) -> U {
        U::fx_from(self)
    }
}

pub trait FXTryFrom<T>: Sized {
    type Error;
    fn fx_try_from(value: T) -> Result<Self, Self::Error>;
}

pub trait FXTryInto<T>: Sized {
    type Error;
    fn fx_try_into(self) -> Result<T, Self::Error>;
}

impl<T, U> FXTryInto<U> for T
where
    U: FXTryFrom<T>,
{
    type Error = U::Error;

    #[inline]
    fn fx_try_into(self) -> Result<U, Self::Error> {
        U::fx_try_from(self)
    }
}

pub trait FXTrigger {
    fn is_true(&self) -> FXProp<bool>;
    fn is_true_opt(&self) -> Option<FXProp<bool>>;
}

/// Base functionality of helper types.
pub trait FXHelperTrait: FXSetState {
    /// Helper method name.
    fn name(&self) -> Option<FXProp<&str>>;
    /// For helper methods that are backed by additional types these are attributes to be applied to the types.
    fn attributes(&self) -> Option<&FXAttributes>;
    /// Additional attributes to apply to generated helper.
    fn attributes_fn(&self) -> Option<&FXAttributes>;
    /// Helper visibility if explicitly set
    fn visibility(&self) -> Option<&syn::Visibility>;
    /// Helper doc if explicitly set.
    fn doc(&self) -> Option<&FXDoc>;
}

impl<T: FXSetState> FXTrigger for Option<T> {
    #[inline]
    fn is_true(&self) -> FXProp<bool> {
        self.as_ref().map_or(FXProp::new(false, None), |h| h.is_set())
    }

    #[inline]
    fn is_true_opt(&self) -> Option<FXProp<bool>> {
        self.as_ref().map(|h| h.is_set())
    }
}

/// Make value traits report their set/unset state. This means:
/// - For types with `off` support unset means `off` is present.
/// - If `off` is ommitted then unset means `None`.
/// - For types that are optional unset means `None`.
pub trait FXSetState {
    fn is_set(&self) -> FXProp<bool>;
    fn is_set_bool(&self) -> bool {
        *self.is_set().value()
    }
}

impl<T> FXSetState for Option<T>
where
    T: FXSetState,
{
    fn is_set(&self) -> FXProp<bool> {
        self.as_ref().map_or(FXProp::new(false, None), |v| v.is_set())
    }
}

impl<T> FXSetState for &T
where
    T: FXSetState,
{
    fn is_set(&self) -> FXProp<bool> {
        T::is_set(*self)
    }
}

impl FXSetState for syn::Visibility {
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(true, Some(self.span()))
    }
}

// Generic trait for all kinds of objects that can report their span.
pub trait FXSpaned {
    fn fx_span(&self) -> Span;
}

impl<T> FXSpaned for T
where
    T: Spanned,
{
    fn fx_span(&self) -> Span {
        self.span()
    }
}
