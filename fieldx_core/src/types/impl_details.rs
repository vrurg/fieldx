pub mod impl_async;
pub mod impl_plain;
pub mod impl_sync;

use std::fmt::Debug;

use darling::Result;
use proc_macro2::Span;
use proc_macro2::TokenStream;

use crate::codegen::constructor::FXFnConstructor;
use crate::ctx::codegen::FXImplementationContext;
use crate::ctx::FXFieldCtx;

pub trait FXImplDetails<ImplCtx>: Debug
where
    ImplCtx: FXImplementationContext,
{
    fn await_call(&self, span: Span) -> TokenStream;
    fn ref_count_strong(&self, span: Span) -> TokenStream;
    fn ref_count_weak(&self, span: Span) -> TokenStream;
    fn field_simple_proxy_type(&self, span: Span) -> TokenStream;
    fn field_lock_proxy_type(&self, span: Span) -> Result<TokenStream>;
    fn fx_mapped_write_guard(&self, span: Span) -> Result<TokenStream>;
    fn fx_fallible_builder_wrapper(&self, span: Span) -> Result<TokenStream>;
    fn fx_infallible_builder_wrapper(&self, span: Span) -> Result<TokenStream>;
    fn lazy_builder(&self, fctx: &FXFieldCtx<ImplCtx>) -> TokenStream;
    fn lazy_wrapper_fn(&self, fctx: &FXFieldCtx<ImplCtx>) -> Result<Option<FXFnConstructor>>;
    fn rwlock(&self, span: Span) -> Result<TokenStream>;
    fn rwlock_mapped_read_guard(&self, span: Span) -> Result<TokenStream>;
    fn rwlock_mapped_write_guard(&self, span: Span) -> Result<TokenStream>;
    fn rwlock_read_guard(&self, span: Span) -> Result<TokenStream>;
    fn rwlock_write_guard(&self, span: Span) -> Result<TokenStream>;
}

impl<ImplCtx> FXImplDetails<ImplCtx> for Box<dyn FXImplDetails<ImplCtx>>
where
    ImplCtx: FXImplementationContext,
{
    fn await_call(&self, span: Span) -> TokenStream {
        self.as_ref().await_call(span)
    }

    fn ref_count_strong(&self, span: Span) -> TokenStream {
        self.as_ref().ref_count_strong(span)
    }

    fn ref_count_weak(&self, span: Span) -> TokenStream {
        self.as_ref().ref_count_weak(span)
    }

    fn field_simple_proxy_type(&self, span: Span) -> TokenStream {
        self.as_ref().field_simple_proxy_type(span)
    }

    fn field_lock_proxy_type(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().field_lock_proxy_type(span)
    }

    fn fx_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().fx_mapped_write_guard(span)
    }

    fn fx_fallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().fx_fallible_builder_wrapper(span)
    }

    fn fx_infallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().fx_infallible_builder_wrapper(span)
    }

    fn lazy_builder(&self, fctx: &FXFieldCtx<ImplCtx>) -> TokenStream {
        self.as_ref().lazy_builder(fctx)
    }

    fn lazy_wrapper_fn(&self, fctx: &FXFieldCtx<ImplCtx>) -> Result<Option<FXFnConstructor>> {
        self.as_ref().lazy_wrapper_fn(fctx)
    }

    fn rwlock(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().rwlock(span)
    }

    fn rwlock_mapped_read_guard(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().rwlock_mapped_read_guard(span)
    }

    fn rwlock_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().rwlock_mapped_write_guard(span)
    }

    fn rwlock_read_guard(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().rwlock_read_guard(span)
    }

    fn rwlock_write_guard(&self, span: Span) -> Result<TokenStream> {
        self.as_ref().rwlock_write_guard(span)
    }
}
