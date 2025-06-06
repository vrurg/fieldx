use darling::Result;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote_spanned;

use crate::codegen::constructor::FXConstructor;
use crate::codegen::constructor::FXFnConstructor;
use crate::ctx::codegen::FXImplementationContext;
use crate::ctx::FXFieldCtx;

use super::FXImplDetails;

#[derive(Debug)]
pub struct FXAsyncImplementor;

impl FXAsyncImplementor {
    fn lazy_wrapper_name<ImplCtx: FXImplementationContext>(&self, fctx: &FXFieldCtx<ImplCtx>) -> syn::Ident {
        let ident = fctx.lazy_ident();
        let span = fctx.lazy().final_span();
        format_ident!("__fx_async_wrap_for_{}", ident, span = span)
    }
}

impl<ImplCtx> FXImplDetails<ImplCtx> for FXAsyncImplementor
where
    ImplCtx: FXImplementationContext,
{
    fn fieldx_impl_mod(&self, span: Span) -> TokenStream {
        quote_spanned! {span=>
            ::fieldx::r#async
        }
    }

    fn field_simple_proxy_type(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::OnceCell]
    }

    fn field_lock_proxy_type(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::FXProxyAsync])
    }

    fn ref_count_strong(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::std::sync::Arc]
    }

    fn ref_count_weak(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::std::sync::Weak]
    }

    fn fx_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::FXWrLockGuardAsync])
    }

    fn fx_fallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::FXBuilderFallible])
    }

    fn fx_infallible_builder_wrapper(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::FXBuilderInfallible])
    }

    fn lazy_wrapper_fn(&self, fctx: &FXFieldCtx<ImplCtx>) -> darling::Result<Option<FXFnConstructor>> {
        let span = fctx.lazy().final_span();
        let lazy_builder_name = fctx.lazy_ident();
        let builder_return = fctx.fallible_return_type(fctx, fctx.ty())?;

        let mut mc = FXFnConstructor::new(self.lazy_wrapper_name(fctx));
        mc.set_span(span)
            .set_ret_type(quote_spanned! {span=> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #builder_return> + Send + '_>>})
            .set_ret_stmt(quote_spanned! {span=>
                ::std::boxed::Box::pin(
                    self.#lazy_builder_name()
                )
            });
        Ok(Some(mc))
    }

    fn lazy_builder(&self, fctx: &FXFieldCtx<ImplCtx>) -> TokenStream {
        let span = fctx.lazy().final_span();
        let input_type = fctx.codegen_ctx().struct_type_toks();
        let wrapper_name = self.lazy_wrapper_name(fctx);
        quote_spanned! {span=> ::std::boxed::Box::new(<#input_type>::#wrapper_name)}
    }

    fn await_call(&self, span: Span) -> TokenStream {
        quote_spanned![span=> .await]
    }

    fn rwlock(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::FXRwLockAsync])
    }

    fn rwlock_read_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::RwLockReadGuard])
    }

    fn rwlock_write_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::RwLockWriteGuard])
    }

    fn rwlock_mapped_read_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::RwLockReadGuard])
    }

    fn rwlock_mapped_write_guard(&self, span: Span) -> Result<TokenStream> {
        Ok(quote_spanned![span=> ::fieldx::r#async::RwLockMappedWriteGuard])
    }
}
