use super::{FXCodeGenSync, FXSyncImplDetails};
use crate::{codegen::FXCodeGenContextual, ctx::FXFieldCtx};
use proc_macro2::TokenStream;
use quote::quote;

pub struct FXSyncImplementor;

impl FXSyncImplDetails for FXSyncImplementor {
    fn field_proxy_type(&self) -> TokenStream {
        quote![::fieldx::sync::FXProxy]
    }

    fn fx_mapped_write_guard(&self) -> TokenStream {
        quote![::fieldx::sync::FXWrLockGuard]
    }

    fn fx_fallible_builder_wrapper(&self) -> TokenStream {
        quote![::fieldx::sync::FXBuilderFallible]
    }

    fn fx_infallible_builder_wrapper(&self) -> TokenStream {
        quote![::fieldx::sync::FXBuilderInfallible]
    }

    fn lazy_wrapper_fn(&self, _: &FXCodeGenSync, _: &FXFieldCtx) -> Result<TokenStream, darling::Error> {
        Ok(quote![])
    }

    fn lazy_builder(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> Result<TokenStream, darling::Error> {
        let input_type = codegen.input_type_toks();
        let lazy_builder_name = codegen.lazy_name(fctx)?;
        Ok(quote![<#input_type>::#lazy_builder_name])
    }

    fn async_decl(&self) -> TokenStream {
        quote![]
    }

    fn await_call(&self) -> TokenStream {
        quote![]
    }

    fn rwlock(&self) -> TokenStream {
        quote![::fieldx::sync::FXRwLock]
    }

    fn rwlock_read_guard(&self) -> TokenStream {
        quote![::fieldx::sync::RwLockReadGuard]
    }

    fn rwlock_write_guard(&self) -> TokenStream {
        quote![::fieldx::sync::RwLockWriteGuard]
    }

    fn rwlock_mapped_read_guard(&self) -> TokenStream {
        quote![::fieldx::sync::MappedRwLockReadGuard]
    }

    fn rwlock_mapped_write_guard(&self) -> TokenStream {
        quote![::fieldx::sync::MappedRwLockWriteGuard]
    }
}
