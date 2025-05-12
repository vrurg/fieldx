use proc_macro2::Span;
use proc_macro2::TokenStream;

use crate::codegen::constructor::FXFnConstructor;
use crate::ctx::FXFieldCtx;

pub mod impl_async;
pub mod impl_sync;

pub trait FXSyncImplDetails {
    fn await_call(&self, span: Span) -> TokenStream;
    fn field_proxy_type(&self, span: Span) -> TokenStream;
    fn fx_mapped_write_guard(&self, span: Span) -> TokenStream;
    fn fx_fallible_builder_wrapper(&self, span: Span) -> TokenStream;
    fn fx_infallible_builder_wrapper(&self, span: Span) -> TokenStream;
    fn lazy_builder(&self, fctx: &FXFieldCtx) -> TokenStream;
    fn lazy_wrapper_fn(&self, fctx: &FXFieldCtx) -> Result<Option<FXFnConstructor>, darling::Error>;
    fn rwlock(&self, span: Span) -> TokenStream;
    fn rwlock_mapped_read_guard(&self, span: Span) -> TokenStream;
    fn rwlock_mapped_write_guard(&self, span: Span) -> TokenStream;
    fn rwlock_read_guard(&self, span: Span) -> TokenStream;
    fn rwlock_write_guard(&self, span: Span) -> TokenStream;
}
