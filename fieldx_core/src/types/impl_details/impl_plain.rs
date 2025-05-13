use darling::Result;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;

use crate::codegen::constructor::FXFnConstructor;
use crate::ctx::codegen::FXImplementationContext;
use crate::ctx::FXFieldCtx;

use super::FXImplDetails;

#[derive(Debug)]
pub struct FXPlainImplementor;

impl<ImplCtx> FXImplDetails<ImplCtx> for FXPlainImplementor
where
    ImplCtx: FXImplementationContext,
{
    fn field_proxy_type(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::OnceCell]
    }

    fn ref_count_strong(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::std::rc::Rc]
    }

    fn ref_count_weak(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::std::rc::Weak]
    }

    fn fx_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("Write guard type is not supported for plain fields").with_span(&span))
    }

    fn fx_fallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("Fallible builder wrapper is not supported for plain fields").with_span(&span))
    }

    fn fx_infallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("Infallible builder wrapper is not supported for plain fields").with_span(&span))
    }

    fn lazy_wrapper_fn(&self, _: &FXFieldCtx<ImplCtx>) -> Result<Option<FXFnConstructor>> {
        Ok(None)
    }

    fn lazy_builder(&self, _fctx: &FXFieldCtx<ImplCtx>) -> TokenStream {
        quote![]
    }

    fn await_call(&self, _span: Span) -> TokenStream {
        quote![]
    }

    fn rwlock(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("RW lock is not supported for plain fields").with_span(&span))
    }

    fn rwlock_read_guard(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("RW lock read guard is not supported for plain fields").with_span(&span))
    }

    fn rwlock_write_guard(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("RW lock write guard is not supported for plain fields").with_span(&span))
    }

    fn rwlock_mapped_read_guard(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("RW lock mapped read guard is not supported for plain fields").with_span(&span))
    }

    fn rwlock_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        Err(darling::Error::custom("RW lock mapped write guard is not supported for plain fields").with_span(&span))
    }
}
