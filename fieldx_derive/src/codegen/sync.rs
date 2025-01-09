mod impl_async;
mod impl_sync;

#[cfg(feature = "serde")]
use crate::codegen::serde::FXCGenSerde;
use crate::codegen::{FXCodeGenContextual, FXCodeGenCtx, FXFieldCtx, FXHelperKind, FXInlining, FXValueRepr};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::spanned::Spanned;

use super::method_constructor::MethodConstructor;

pub trait FXSyncImplDetails {
    fn is_async(&self) -> bool;
    fn await_call(&self) -> TokenStream;
    fn field_proxy_type(&self) -> TokenStream;
    fn fx_mapped_write_guard(&self) -> TokenStream;
    fn fx_fallible_builder_wrapper(&self) -> TokenStream;
    fn fx_infallible_builder_wrapper(&self) -> TokenStream;
    fn lazy_builder(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> Result<TokenStream, darling::Error>;
    fn lazy_wrapper_fn(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> Result<TokenStream, darling::Error>;
    fn rwlock(&self) -> TokenStream;
    fn rwlock_mapped_read_guard(&self) -> TokenStream;
    fn rwlock_mapped_write_guard(&self) -> TokenStream;
    fn rwlock_read_guard(&self) -> TokenStream;
    fn rwlock_write_guard(&self) -> TokenStream;
}

pub struct FXCodeGenSync<'a> {
    codegen:    &'a crate::codegen::FXRewriter<'a>,
    ctx:        Rc<FXCodeGenCtx>,
    impl_async: impl_async::FXAsyncImplementor,
    impl_sync:  impl_sync::FXSyncImplementor,
}

impl<'a> FXCodeGenSync<'a> {
    pub fn new(codegen: &'a crate::codegen::FXRewriter<'a>, ctx: Rc<FXCodeGenCtx>) -> Self {
        Self {
            codegen,
            ctx,
            impl_async: impl_async::FXAsyncImplementor,
            impl_sync: impl_sync::FXSyncImplementor,
        }
    }

    #[inline]
    fn implementor(&self, fctx: &FXFieldCtx) -> &dyn FXSyncImplDetails {
        if fctx.is_async() {
            &self.impl_async
        }
        else {
            &self.impl_sync
        }
    }

    fn read_method_name(&self, fctx: &FXFieldCtx, mutable: bool, span: &Span) -> syn::Ident {
        let sfx = if mutable { "_mut" } else { "" };
        if fctx.is_fallible() {
            format_ident!("try_read{sfx}", span = span.clone())
        }
        else {
            format_ident!("read{sfx}", span = span.clone())
        }
    }

    fn field_reader_method(&self, fctx: &FXFieldCtx, helper_kind: FXHelperKind) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, helper_kind);
        let mut mc = MethodConstructor::new(self.helper_name(fctx, helper_kind)?);
        let ident = fctx.ident_tok();
        let ty = fctx.ty();
        let implementor = self.implementor(fctx);
        let rwlock_guard = implementor.rwlock_read_guard();
        let await_call = implementor.await_call();
        let read_method = self.read_method_name(fctx, false, &span);
        let lifetime = quote_spanned! {span=> 'fx_reader_lifetime};

        self.maybe_ref_counted_self(fctx, &mut mc);
        mc.set_vis(fctx.vis_tok(helper_kind));
        mc.set_span(span);
        mc.set_async(fctx.is_async());
        mc.maybe_add_attribute(self.attributes_fn(fctx, helper_kind, FXInlining::Always));

        if fctx.is_lazy() {
            let mapped_guard = self.implementor(fctx).rwlock_mapped_read_guard();
            let self_rc = mc.self_maybe_rc();

            mc.set_self_lifetime(lifetime.clone());
            mc.set_ret_type(self.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<#lifetime, #ty> })?);
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method(&#self_rc)#await_call});
        }
        else if fctx.is_optional() {
            mc.set_ret_type(quote_spanned! {span=> #rwlock_guard<#lifetime, ::std::option::Option<#ty>> });
            mc.set_self_lifetime(lifetime);
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method()#await_call});
        }
        else {
            mc.set_ret_type(quote_spanned! {span=> #rwlock_guard<#ty> });
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.read()#await_call});
        }

        Ok(mc.into_method())
    }

    #[inline(always)]
    fn maybe_optional_ty<T: ToTokens>(&self, fctx: &FXFieldCtx, ty: &T) -> TokenStream {
        if fctx.is_optional() {
            let span = fctx.optional_span();
            quote_spanned![span=> ::std::option::Option<#ty>]
        }
        else {
            quote![#ty]
        }
    }

    #[inline(always)]
    fn maybe_locked_ty<T: ToTokens>(&self, fctx: &FXFieldCtx, ty: &T) -> TokenStream {
        if fctx.needs_lock() {
            let span = fctx.lock_span();
            let rwlock = self.implementor(fctx).rwlock();
            quote_spanned![span=> #rwlock<#ty>]
        }
        else {
            quote![#ty]
        }
    }

    fn input_type_toks(&self) -> TokenStream {
        let ident = self.ctx().input_ident();
        let generic_params = self.generic_params();
        quote::quote! {
            #ident #generic_params
        }
    }

    fn builder_wrapper_type(&self, fctx: &FXFieldCtx, turbo_fish: bool) -> darling::Result<TokenStream> {
        let ty = fctx.ty();
        let input_type = self.input_type_toks();
        let dcolon = if turbo_fish { quote![::] } else { quote![] };
        let (wrapper_type, error_type) = if fctx.is_fallible() {
            let error_type = fctx.fallible_error()?;
            (
                self.implementor(fctx).fx_fallible_builder_wrapper(),
                quote![, #error_type],
            )
        }
        else {
            (self.implementor(fctx).fx_infallible_builder_wrapper(), quote![])
        };

        let span = fctx.fallible_span();

        Ok(quote_spanned! {span=> #wrapper_type #dcolon <#input_type, #ty #error_type>})
    }

    fn wrap_builder(&self, fctx: &FXFieldCtx, builder: TokenStream) -> darling::Result<TokenStream> {
        let wrapper_type = self.builder_wrapper_type(fctx, true)?;
        let span = fctx.fallible_span();
        Ok(quote_spanned! {span=>
            #wrapper_type::new(#builder)
        })
    }
}

impl<'a> FXCodeGenContextual for FXCodeGenSync<'a> {
    #[inline(always)]
    fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.ctx
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> darling::Result<&'s TokenStream> {
        let builder_wrapper_type = self.builder_wrapper_type(fctx, false)?;
        Ok(fctx.ty_wrapped(|| {
            let ty = fctx.ty_tok().clone();

            if fctx.is_skipped() {
                return ty;
            }

            if fctx.is_lazy() {
                let proxy_type = self.implementor(fctx).field_proxy_type();
                let span = fctx.ty().span();
                quote_spanned! [span=> #proxy_type<#builder_wrapper_type>]
            }
            else {
                self.maybe_locked_ty(fctx, &self.maybe_optional_ty(fctx, &ty))
            }
        }))
    }

    fn ref_count_types(&self) -> (TokenStream, TokenStream) {
        (quote![::std::sync::Arc], quote![::std::sync::Weak])
    }

    fn field_lazy_builder_wrapper(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.is_lazy() {
            self.implementor(fctx).lazy_wrapper_fn(self, fctx)
        }
        else {
            Ok(quote![])
        }
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor() {
            let is_copy = fctx.is_copy();
            let is_clone = fctx.is_clone();
            let is_lazy = fctx.is_lazy();

            if !(is_copy || is_clone) && (is_lazy || fctx.needs_lock()) {
                return self.field_reader_method(fctx, FXHelperKind::Accessor);
            }

            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let mut mc = MethodConstructor::new(self.accessor_name(fctx)?);
            let is_optional = fctx.is_optional();
            let ident = fctx.ident_tok();
            let ty = fctx.ty();
            let read_method = self.read_method_name(fctx, false, &span);

            self.maybe_ref_counted_self(fctx, &mut mc);
            mc.set_span(span);
            mc.set_vis(fctx.vis_tok(FXHelperKind::Accessor));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Accessor, FXInlining::Always));

            if is_clone || is_copy {
                // unwrap won't panic because somewhere out there a copy/clone argument exists.
                let cc_span = fctx.accessor_mode_span().unwrap();
                let shortcut = fctx.fallible_shortcut();
                let cmethod = if is_copy {
                    if is_optional {
                        quote_spanned![cc_span=> .as_ref().copied()]
                    }
                    else {
                        quote![]
                    }
                }
                else {
                    if is_optional {
                        quote_spanned![cc_span=> .as_ref().cloned()]
                    }
                    else {
                        quote_spanned![cc_span=> .clone()]
                    }
                };

                if is_lazy {
                    let implementor = self.implementor(fctx);
                    let await_call = implementor.await_call();
                    let self_rc = mc.self_maybe_rc();
                    let ty = self.fallible_return_type(fctx, ty)?;

                    mc.set_async(fctx.is_async());
                    mc.set_ret_type(ty);
                    mc.add_statement(
                        quote_spanned! {span=> let rlock = self.#ident.#read_method(&#self_rc) #await_call #shortcut; },
                    );
                    mc.set_ret_stmt(fctx.fallible_ok_return(quote_spanned! {span=> (*rlock)#cmethod}));
                }
                else if fctx.needs_lock() {
                    mc.set_ret_type(self.maybe_optional_ty(fctx, ty));
                    mc.add_statement(quote_spanned! {span=> let rlock = self.#ident.read() #shortcut; });
                    mc.set_ret_stmt(quote_spanned! {span=> (*rlock)#cmethod});
                }
                else if is_optional {
                    mc.set_ret_type(self.maybe_optional_ty(fctx, ty));
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident #cmethod });
                }
                else {
                    let cmethod = if fctx.is_copy() { quote![] } else { quote![.clone()] };
                    mc.set_ret_type(ty.to_token_stream());
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident #cmethod });
                }
            }
            else {
                let ty = self.fallible_return_type(fctx, &self.maybe_optional_ty(fctx, ty))?;

                mc.set_ret_type(quote_spanned! {span=> &#ty });
                mc.set_ret_stmt(quote_spanned! {span=> &self.#ident });
            }

            mc.into_method()
        }
        else {
            quote![]
        })
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor_mut() {
            let mut mc = MethodConstructor::new(self.accessor_mut_name(fctx)?);
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let ident = fctx.ident_tok();
            let ty = fctx.ty();
            let read_method = self.read_method_name(fctx, true, &span);

            self.maybe_ref_counted_self(fctx, &mut mc);
            mc.set_span(span);
            mc.set_vis(fctx.vis_tok(FXHelperKind::Accessor));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Accessor, FXInlining::Always));

            if fctx.is_lazy() {
                let implementor = self.implementor(fctx);
                let self_rc = mc.self_maybe_rc();
                let await_call = implementor.await_call();
                let mapped_guard = self.implementor(fctx).rwlock_mapped_write_guard();
                let lifetime = quote_spanned! {span=> 'fx_get_mut};

                mc.set_async(fctx.is_async());
                mc.set_self_lifetime(lifetime.clone());
                mc.set_ret_type(
                    self.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<#lifetime, #ty> })?,
                );
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method(&#self_rc) #await_call });
            }
            else {
                let ty_toks = if fctx.is_optional() {
                    let opt_span = fctx.optional_span();
                    quote_spanned![opt_span=> ::std::option::Option<#ty>]
                }
                else {
                    ty.to_token_stream()
                };
                if fctx.needs_lock() {
                    let lock_span = fctx.lock_span();
                    let wrguard = self.implementor(fctx).rwlock_write_guard();

                    mc.set_ret_type(quote_spanned! [lock_span=> #wrguard<#ty_toks>]);
                    mc.set_ret_stmt(quote_spanned! [lock_span=> self.#ident.write()]);
                }
                else {
                    // Bare field
                    return self.codegen.plain_gen().field_accessor_mut(fctx);
                    // mc.set_ret_type(quote_spanned! {span=> &mut #ty});
                    // mc.set_ret_stmt(quote_spanned! {span=> &mut self.#ident });
                }
            }

            mc.into_method()
        }
        else {
            quote![]
        })
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, FXHelperKind::Builder);
        let field_ident = fctx.ident_tok();
        let field_type = self.type_tokens(fctx)?;
        let is_optional = fctx.is_optional();
        let is_lazy = fctx.is_lazy();
        let default = self.field_default_value(fctx);
        let lazy_builder = self.implementor(fctx).lazy_builder(self, fctx)?;
        let or_default = if fctx.has_default_value() {
            let default = self.fixup_self_type(default.clone().expect(&format!(
                "Internal problem: expected default value for field {}",
                fctx.ident_str()
            )));

            if is_optional || is_lazy {
                quote_spanned! [span=> .or_else(|| ::std::option::Option::Some(#default))]
            }
            else {
                quote_spanned! [span=> .unwrap_or_else(|| #default)]
            }
        }
        else {
            quote![]
        };

        Ok(if !fctx.forced_builder() && !fctx.needs_builder() {
            let default = self.field_value_wrap(fctx, default)?;
            quote_spanned![span=> #field_ident: #default]
        }
        else if fctx.is_lazy() {
            let lazy_builder = self.wrap_builder(fctx, lazy_builder)?;
            quote_spanned![span=>
                #field_ident: <#field_type>::new_default(
                    #lazy_builder,
                    self.#field_ident.take()#or_default
                )
            ]
        }
        else if fctx.needs_lock() {
            let set_value = self.field_builder_value_for_set(fctx, field_ident, &span);
            quote_spanned![span=>
                // Optional can simply pickup and own either Some() or None from the builder.
                #field_ident: #set_value
            ]
        }
        else if is_optional {
            quote_spanned! [span=>
                #field_ident: self.#field_ident.take()#or_default
            ]
        }
        else {
            self.simple_field_build_setter(fctx, field_ident, &span)
        })
    }

    fn field_lazy_initializer(&self, _fctx: &FXFieldCtx, mc: &mut MethodConstructor) -> darling::Result<TokenStream> {
        let self_var = mc.self_maybe_rc();
        Ok(quote![.lazy_init(&#self_var)])
    }

    #[cfg(feature = "serde")]
    fn field_from_shadow(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let shadow_var = self.ctx().shadow_var_ident();
        self.field_value_wrap(fctx, FXValueRepr::Exact(quote![#shadow_var.#field_ident ]))
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let me_var = self.ctx().me_var_ident();
        Ok(if self.is_serde_optional(fctx) || fctx.needs_lock() {
            quote![ #me_var.#field_ident.into_inner() ]
        }
        else {
            quote![ #me_var.#field_ident ]
        })
    }

    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_reader() {
            self.field_reader_method(fctx, FXHelperKind::Reader)
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_writer() {
            let span = self.helper_span(fctx, FXHelperKind::Writer);
            let mut mc = MethodConstructor::new(self.writer_name(fctx)?);
            let ident = fctx.ident_tok();
            let mut ret_ty = fctx.ty().to_token_stream();
            let await_call = self.implementor(fctx).await_call();

            mc.set_vis(fctx.vis_tok(FXHelperKind::Writer));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Writer, FXInlining::Always));
            mc.set_async(self.implementor(fctx).is_async());

            if fctx.is_lazy() {
                let lazy_span = fctx.helper_span(FXHelperKind::Lazy);
                let lifetime = quote_spanned! {lazy_span=> 'fx_writer_lifetime};
                let fx_wrlock_guard = self.implementor(fctx).fx_mapped_write_guard();
                let builder_wrapper_type = self.builder_wrapper_type(fctx, false)?;

                mc.set_self_lifetime(lifetime.clone());
                mc.set_ret_type(quote_spanned! {span=> #fx_wrlock_guard<#lifetime, #builder_wrapper_type>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call});
            }
            else {
                let wrlock_guard = self.implementor(fctx).rwlock_write_guard();
                if fctx.is_optional() {
                    ret_ty = quote_spanned! {span=> ::std::option::Option<#ret_ty>};
                }

                mc.set_ret_type(quote_spanned! {span=> #wrlock_guard<#ret_ty>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call});
            }

            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_setter() {
            let span = self.helper_span(fctx, FXHelperKind::Setter);
            let mut mc = MethodConstructor::new(self.setter_name(fctx)?);
            let ident = fctx.ident_tok();
            let ty = fctx.ty();
            let (val_type, gen_params, into_tok) = self.into_toks(fctx, fctx.is_setter_into());
            let await_call = self.implementor(fctx).await_call();
            let value_tok = quote_spanned! {span=> value #into_tok};
            let is_lazy = fctx.is_lazy();
            let is_optional = fctx.is_optional();
            let needs_lock = fctx.needs_lock();

            mc.set_span(span);
            mc.set_vis(fctx.vis_tok(FXHelperKind::Setter));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Setter, FXInlining::Always));
            mc.maybe_add_generic(gen_params);
            mc.add_param(quote_spanned! {span=> value: #val_type});

            if is_lazy || is_optional || needs_lock {
                mc.set_async(self.implementor(fctx).is_async());
            }

            if is_lazy {
                mc.set_ret_type(quote_spanned! {span=> ::std::option::Option<#ty>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call.store(#value_tok)});
            }
            else if is_optional {
                let opt_span = fctx.optional_span();
                let (lock_method, opt_await_call) = if needs_lock {
                    let lock_span = fctx.lock().span();
                    (quote_spanned! {lock_span=> .write()}, await_call)
                }
                else {
                    mc.set_self_mut(true);

                    (quote![], quote![])
                };

                mc.set_ret_type(quote_spanned! {opt_span=> ::std::option::Option<#ty>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident #lock_method #opt_await_call.replace(#value_tok)});
            }
            else if needs_lock {
                mc.set_ret_type(ty.to_token_stream());
                mc.add_statement(quote_spanned! {span=> let mut wlock = self.#ident.write()#await_call; });
                mc.set_ret_stmt(quote_spanned! {span=> ::std::mem::replace(&mut *wlock, #value_tok)});
            }
            else {
                mc.set_ret_type(ty.to_token_stream());
                mc.set_self_mut(true);
                mc.set_ret_stmt(quote_spanned! {span=> ::std::mem::replace(&mut self.#ident, #value_tok)});
            }

            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_clearer() {
            let span = self.helper_span(fctx, FXHelperKind::Clearer);
            let mut mc = MethodConstructor::new(self.clearer_name(fctx)?);
            let ident = fctx.ident_tok();
            let ty = fctx.ty();
            let await_call = self.implementor(fctx).await_call();

            mc.set_vis(fctx.vis_tok(FXHelperKind::Clearer));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Clearer, FXInlining::Always));
            mc.set_async(self.implementor(fctx).is_async());
            mc.set_ret_type(quote_spanned! {span=> ::std::option::Option<#ty>});

            if fctx.is_lazy() {
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.clear()#await_call});
            }
            else {
                // If not lazy then it's optional
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call.take()});
            }

            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_predicate() {
            let span = self.helper_span(fctx, FXHelperKind::Predicate);
            let mut mc = MethodConstructor::new(self.predicate_name(fctx)?);
            let ident = fctx.ident_tok();

            mc.set_vis(fctx.vis_tok(FXHelperKind::Predicate));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Predicate, FXInlining::Always));
            mc.set_ret_type(quote_spanned! {span=> bool});

            if fctx.is_lazy() {
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.is_set()});
            }
            // If not lazy then it's optional
            else if fctx.needs_lock() {
                mc.set_async(self.implementor(fctx).is_async());
                let await_call = self.implementor(fctx).await_call();
                let read_method = self.read_method_name(fctx, false, &span);
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method()#await_call.is_some()});
            }
            else {
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.is_some()});
            }

            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: FXValueRepr<TokenStream>) -> darling::Result<TokenStream> {
        let ident_span = fctx.ident().span();
        let is_optional = fctx.is_optional();
        let is_lazy = fctx.is_lazy();

        let value_wrapper = match &value {
            FXValueRepr::None => quote_spanned![ident_span=> ::std::option::Option::None],
            FXValueRepr::Exact(v) => quote_spanned![ident_span=> #v],
            FXValueRepr::Versatile(v) => {
                if is_optional || is_lazy {
                    quote_spanned![ident_span=> ::std::option::Option::Some(#v)]
                }
                else {
                    quote_spanned![ident_span=> #v]
                }
            }
        };

        Ok(if is_lazy {
            let field_type = self.type_tokens(fctx)?;
            let lazy_builder = self.wrap_builder(fctx, self.implementor(fctx).lazy_builder(self, fctx)?)?;

            quote_spanned![ident_span=> <#field_type>::new_default(#lazy_builder, #value_wrapper) ]
        }
        else {
            let rwlock = self.implementor(fctx).rwlock();

            if !is_optional && value.is_none() {
                return Err(darling::Error::custom(format!(
                    "No value was supplied for non-optional, non-lazy field '{}'",
                    fctx.ident_str()
                )));
            }

            if fctx.needs_lock() {
                quote_spanned![ident_span=> #rwlock::new(#value_wrapper) ]
            }
            else {
                value_wrapper
            }
        })
    }

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.field_value_wrap(fctx, self.field_default_value(fctx))
    }
}
