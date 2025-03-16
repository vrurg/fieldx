use super::{FXCodeGenSync, FXSyncImplDetails};
use crate::{codegen::constructor::FXFnConstructor, ctx::FXFieldCtx};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};

pub(crate) struct FXSyncImplementor;

impl FXSyncImplDetails for FXSyncImplementor {
    fn field_proxy_type(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::FXProxySync]
    }

    fn fx_mapped_write_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::FXWrLockGuardSync]
    }

    fn fx_fallible_builder_wrapper(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::FXBuilderFallible]
    }

    fn fx_infallible_builder_wrapper(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::FXBuilderInfallible]
    }

    fn lazy_wrapper_fn(&self, _: &FXCodeGenSync, _: &FXFieldCtx) -> Result<Option<FXFnConstructor>, darling::Error> {
        Ok(None)
    }

    fn lazy_builder(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> TokenStream {
        let input_type = codegen.input_type_toks();
        let lazy_builder_name = fctx.lazy_ident();
        let span = fctx.lazy().final_span();
        quote_spanned![span=> <#input_type>::#lazy_builder_name]
    }

    fn await_call(&self, _span: Span) -> TokenStream {
        quote![]
    }

    fn rwlock(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::FXRwLockSync]
    }

    fn rwlock_read_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::RwLockReadGuard]
    }

    fn rwlock_write_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::RwLockWriteGuard]
    }

    fn rwlock_mapped_read_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::MappedRwLockReadGuard]
    }

    fn rwlock_mapped_write_guard(&self, span: Span) -> TokenStream {
        quote_spanned![span=> ::fieldx::sync::MappedRwLockWriteGuard]
    }
}
