use super::{FXCodeGenSync, FXSyncImplDetails};
use crate::{
    codegen::{self, FXCodeGenContextual},
    ctx::FXFieldCtx,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};

pub struct FXAsyncImplementor;

impl FXAsyncImplementor {
    fn lazy_wrapper_name(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> darling::Result<syn::Ident> {
        let ident = codegen.helper_name(fctx, codegen::FXHelperKind::Lazy)?;
        let span = codegen.helper_span(fctx, codegen::FXHelperKind::Lazy);
        Ok(format_ident!("__fx_async_wrap_for_{}", ident, span = span))
    }
}

impl FXSyncImplDetails for FXAsyncImplementor {
    fn field_proxy_type(&self) -> TokenStream {
        quote![::fieldx::r#async::FXProxyAsync]
    }

    fn fx_mapped_write_guard(&self) -> TokenStream {
        quote![::fieldx::r#async::FXWrLockGuardAsync]
    }

    fn fx_fallible_builder_wrapper(&self) -> TokenStream {
        quote![::fieldx::r#async::FXBuilderFallible]
    }

    fn fx_infallible_builder_wrapper(&self) -> TokenStream {
        quote![::fieldx::r#async::FXBuilderInfallible]
    }

    fn lazy_wrapper_fn(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = fctx.helper_span(super::FXHelperKind::Lazy);
        let wrapper_name = self.lazy_wrapper_name(codegen, fctx)?;
        let lazy_builder_name = codegen.lazy_name(fctx)?;
        let builder_return = codegen.fallible_return_type(fctx, fctx.ty())?;
        Ok(
            quote_spanned![span=> fn #wrapper_name(&self) -> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #builder_return> + Send + '_>> {
                ::std::boxed::Box::pin(
                    self.#lazy_builder_name()
                )
            }],
        )
    }

    fn lazy_builder(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = fctx.helper_span(super::FXHelperKind::Lazy);
        let input_type = codegen.input_type_toks();
        let wrapper_name = self.lazy_wrapper_name(codegen, fctx)?;
        Ok(quote_spanned![span=> ::std::boxed::Box::new(<#input_type>::#wrapper_name)])
    }

    fn is_async(&self) -> bool {
        true
    }

    fn await_call(&self) -> TokenStream {
        quote![.await]
    }

    fn rwlock(&self) -> TokenStream {
        quote![::fieldx::r#async::FXRwLockAsync]
    }

    fn rwlock_read_guard(&self) -> TokenStream {
        quote![::fieldx::r#async::RwLockReadGuard]
    }

    fn rwlock_write_guard(&self) -> TokenStream {
        quote![::fieldx::r#async::RwLockWriteGuard]
    }

    fn rwlock_mapped_read_guard(&self) -> TokenStream {
        quote![::fieldx::r#async::RwLockReadGuard]
    }

    fn rwlock_mapped_write_guard(&self) -> TokenStream {
        quote![::fieldx::r#async::RwLockMappedWriteGuard]
    }
}
