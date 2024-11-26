use proc_macro2::Span;
use syn::{self, spanned::Spanned};

pub trait FXOrig<O>
where
    O: Spanned,
{
    #[allow(dead_code)]
    fn orig(&self) -> Option<&O>;

    #[allow(dead_code)]
    fn orig_span(&self) -> Option<Span> {
        self.orig().and_then(|o| Some(o.span()))
    }

    fn fx_span(&self) -> Span {
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

    fn fx_span(&self) -> Span {
        self.as_ref().map_or_else(|| Span::call_site(), |s| s.fx_span())
    }
}
