use super::FXCodeGenSync;
use super::FXSyncImplDetails;
use crate::codegen::constructor::FXConstructor;
use crate::codegen::constructor::FXFnConstructor;
use crate::codegen::FXCodeGenContextual;
use crate::ctx::FXFieldCtx;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote_spanned;

pub(crate) struct FXAsyncImplementor;

impl FXAsyncImplementor {
    fn lazy_wrapper_name(&self, fctx: &FXFieldCtx) -> syn::Ident {
        let ident = fctx.lazy_ident();
        let span = fctx.lazy().final_span();
        format_ident!("__fx_async_wrap_for_{}", ident, span = span)
    }
}

impl FXSyncImplDetails for FXAsyncImplementor {
    fn field_proxy_type(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::FXProxyAsync]
    }

    fn fx_mapped_write_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::FXWrLockGuardAsync]
    }

    fn fx_fallible_builder_wrapper(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::FXBuilderFallible]
    }

    fn fx_infallible_builder_wrapper(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::FXBuilderInfallible]
    }

    fn lazy_wrapper_fn(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let span = fctx.lazy().final_span();
        let lazy_builder_name = fctx.lazy_ident();
        let builder_return = codegen.fallible_return_type(fctx, fctx.ty())?;

        let mut mc = FXFnConstructor::new(self.lazy_wrapper_name(fctx));
        mc.set_span(span)
            .set_ret_type(quote_spanned! {span=> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #builder_return> + Send + '_>>})
            .set_ret_stmt(quote_spanned! {span=>
                ::std::boxed::Box::pin(
                    self.#lazy_builder_name()
                )
            });
        // Ok(quote_spanned! {span=>
        //     fn #wrapper_name(&self)
        //         -> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #builder_return> + Send + '_>>
        //     {
        //         ::std::boxed::Box::pin(
        //             self.#lazy_builder_name()
        //         )
        //     }
        // })
        Ok(Some(mc))
    }

    fn lazy_builder(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> TokenStream {
        let span = fctx.lazy().final_span();
        let input_type = codegen.input_type_toks();
        let wrapper_name = self.lazy_wrapper_name(fctx);
        quote_spanned! {span=> ::std::boxed::Box::new(<#input_type>::#wrapper_name)}
    }

    fn await_call(&self, span: Span) -> TokenStream {
        quote_spanned![span=> .await]
    }

    fn rwlock(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::FXRwLockAsync]
    }

    fn rwlock_read_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::RwLockReadGuard]
    }

    fn rwlock_write_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::RwLockWriteGuard]
    }

    fn rwlock_mapped_read_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::RwLockReadGuard]
    }

    fn rwlock_mapped_write_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::r#async::RwLockMappedWriteGuard]
    }
}
