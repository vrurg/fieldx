use proc_macro2::Span;
use syn::{self, spanned::Spanned};

pub trait FXOrig<O>
where
    O: Spanned,
{
    #[allow(dead_code)]
    fn orig(&self) -> Option<&O>;

    #[allow(dead_code)]
    fn span(&self) -> Option<Span> {
        self.orig().and_then(|o| Some(o.span()))
    }
}
