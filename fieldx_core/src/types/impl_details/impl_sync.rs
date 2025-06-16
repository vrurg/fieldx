use crate::codegen::constructor::FXFnConstructor;
use crate::ctx::codegen::FXImplementationContext;
use crate::ctx::FXFieldCtx;

use darling::Result;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;

use super::FXImplDetails;

#[derive(Debug)]
pub struct FXSyncImplementor;

impl<ImplCtx> FXImplDetails<ImplCtx> for FXSyncImplementor
where
    ImplCtx: FXImplementationContext,
{
    fn fieldx_impl_mod(&self, span: Span) -> TokenStream {
        quote_spanned! {span=>
            ::fieldx::sync
        }
    }

    fn field_simple_proxy_type(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::OnceCell]
    }

    fn field_lock_proxy_type(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXProxySync])
    }

    fn ref_count_strong(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::std::sync::Arc]
    }

    fn ref_count_weak(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::std::sync::Weak]
    }

    fn fx_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXWrLockGuardSync])
    }

    fn fx_fallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXBuilderFallible])
    }

    fn fx_infallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXBuilderInfallible])
    }

    fn lazy_wrapper_fn(&self, _: &FXFieldCtx<ImplCtx>) -> Result<Option<FXFnConstructor>> {
        Ok(None)
    }

    fn lazy_builder(&self, fctx: &FXFieldCtx<ImplCtx>) -> TokenStream {
        let ctx = fctx.codegen_ctx();
        let input_type = ctx.struct_type_toks();
        let lazy_builder_name = fctx.lazy_ident();
        let span = fctx.lazy().final_span();
        quote_spanned![span=> <#input_type>::#lazy_builder_name]
    }

    fn await_call(&self, _span: Span) -> TokenStream {
        quote![]
    }

    fn rwlock(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXRwLockSync])
    }

    fn rwlock_read_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::RwLockReadGuard])
    }

    fn rwlock_write_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::RwLockWriteGuard])
    }

    fn rwlock_mapped_read_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXProxyReadGuard])
    }

    fn rwlock_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::sync::FXProxyWriteGuard])
    }
}
