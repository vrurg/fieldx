//! Trait for objects that know their origins.
use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{self};

/// For types that "know" their origins.
pub trait FXOrig<O>
where
    O: Spanned,
{
    /// Return the original syntax element.
    #[allow(dead_code)]
    fn orig(&self) -> Option<&O>;

    /// Span of the original syntax element.
    #[allow(dead_code)]
    fn orig_span(&self) -> Option<Span> {
        self.orig().map(|o| o.span())
    }

    /// If there is original syntax element then its span is returned. Otherwise call site is used.
    fn final_span(&self) -> Span {
        #[allow(clippy::redundant_closure)]
        self.orig_span().unwrap_or_else(|| Span::call_site())
    }
}

impl<O, T> FXOrig<O> for Option<T>
where
    O: Spanned,
    T: FXOrig<O>,
{
    fn orig(&self) -> Option<&O> {
        self.as_ref().and_then(|s| s.orig())
    }

    fn orig_span(&self) -> Option<Span> {
        self.as_ref().and_then(|s| s.orig_span())
    }

    fn final_span(&self) -> Span {
        #[allow(clippy::redundant_closure)]
        self.as_ref().map_or_else(|| Span::call_site(), |s| s.final_span())
    }
}

impl<O, T> FXOrig<O> for &T
where
    O: Spanned,
    T: FXOrig<O>,
{
    fn orig(&self) -> Option<&O> {
        (*self).orig()
    }

    fn orig_span(&self) -> Option<Span> {
        (*self).orig_span()
    }

    fn final_span(&self) -> Span {
        (*self).final_span()
    }
}
