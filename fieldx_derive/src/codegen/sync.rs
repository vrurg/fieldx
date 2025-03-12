mod impl_async;
mod impl_sync;

use crate::codegen::{FXCodeGenContextual, FXCodeGenCtx, FXFieldCtx, FXHelperKind, FXInlining, FXValueRepr};
#[allow(unused)]
use crate::util::dump_tt_struct;
use fieldx_aux::FXPropBool;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::spanned::Spanned;

use super::constructor::method::MethodConstructor;

pub trait FXSyncImplDetails {
    fn await_call(&self, span: Span) -> TokenStream;
    fn field_proxy_type(&self, span: Span) -> TokenStream;
    fn fx_mapped_write_guard(&self, span: Span) -> TokenStream;
    fn fx_fallible_builder_wrapper(&self, span: Span) -> TokenStream;
    fn fx_infallible_builder_wrapper(&self, span: Span) -> TokenStream;
    fn lazy_builder(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> TokenStream;
    fn lazy_wrapper_fn(&self, codegen: &FXCodeGenSync, fctx: &FXFieldCtx) -> Result<TokenStream, darling::Error>;
    fn rwlock(&self, span: Span) -> TokenStream;
    fn rwlock_mapped_read_guard(&self, span: Span) -> TokenStream;
    fn rwlock_mapped_write_guard(&self, span: Span) -> TokenStream;
    fn rwlock_read_guard(&self, span: Span) -> TokenStream;
    fn rwlock_write_guard(&self, span: Span) -> TokenStream;
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
        if *fctx.mode_async() {
            &self.impl_async
        }
        else {
            &self.impl_sync
        }
    }

    fn read_method_name(&self, fctx: &FXFieldCtx, mutable: bool, span: Span) -> syn::Ident {
        let sfx = if mutable { "_mut" } else { "" };
        if *fctx.fallible() {
            format_ident!("try_read{sfx}", span = span)
        }
        else {
            format_ident!("read{sfx}", span = span)
        }
    }

    fn field_reader_method(
        &self,
        fctx: &FXFieldCtx,
        helper_ident: &syn::Ident,
        helper_vis: &syn::Visibility,
        attributes_fn: TokenStream,
        span: Span,
    ) -> darling::Result<TokenStream> {
        let mut helper_ident = helper_ident.clone();
        helper_ident.set_span(span);
        let mut mc = MethodConstructor::new(helper_ident);
        let ident = fctx.ident();
        let ty = fctx.ty();
        let implementor = self.implementor(fctx);
        let rwlock_guard = implementor.rwlock_read_guard(span);
        let await_call = implementor.await_call(span);
        let read_method = self.read_method_name(fctx, false, span);
        let lifetime = quote_spanned! {span=> 'fx_reader_lifetime};

        self.maybe_ref_counted_self(fctx, &mut mc);
        mc.set_vis(helper_vis.clone());
        mc.set_span(span);
        mc.set_async(fctx.mode_async());
        mc.add_attribute(attributes_fn);

        let lazy = fctx.lazy();
        let optional = fctx.optional();

        // Tokens for set_ret_type and set_ret_stmt are generated using the default span because the components of the
        // return type that are related to specific arguments (lazy, optional) are already bound to their respective
        // spans.  However, their surrounding syntax belongs to the method itself.
        if *lazy {
            let lazy_span = lazy.final_span();
            let mapped_guard = self.implementor(fctx).rwlock_mapped_read_guard(lazy_span);
            let self_rc = mc.self_maybe_rc();

            mc.set_self_lifetime(lifetime.clone());
            mc.set_ret_type(self.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<#lifetime, #ty> })?);
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method(&#self_rc)#await_call});
        }
        else if *optional {
            let opt_span = optional.final_span();
            let ty = quote_spanned![opt_span=> ::std::option::Option<#ty>];
            mc.set_ret_type(quote_spanned! {span=> #rwlock_guard<#lifetime, #ty> });
            mc.set_self_lifetime(lifetime);
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method()#await_call});
        }
        else {
            mc.set_ret_type(quote_spanned! {span=> #rwlock_guard<#ty> });
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.read()#await_call});
        }

        Ok(mc.to_method())
    }

    #[inline(always)]
    fn maybe_optional_ty<T: ToTokens>(&self, fctx: &FXFieldCtx, ty: &T) -> TokenStream {
        let optional = fctx.optional();
        if *optional {
            let span = optional.final_span();
            quote_spanned![span=> ::std::option::Option<#ty>]
        }
        else {
            ty.to_token_stream()
        }
    }

    #[inline(always)]
    fn maybe_locked_ty<T: ToTokens>(&self, fctx: &FXFieldCtx, ty: &T) -> TokenStream {
        let lock = fctx.lock();
        if *lock {
            let span = lock.final_span();
            let rwlock = self.implementor(fctx).rwlock(span);
            quote_spanned![span=> #rwlock<#ty>]
        }
        else {
            ty.to_token_stream()
        }
    }

    fn input_type_toks(&self) -> TokenStream {
        let ident = self.ctx().input_ident();
        let generic_params = self.generic_params();
        quote_spanned! {ident.span()=>
            #ident #generic_params
        }
    }

    // Compose declaration of the type to use for holding the builder method object.
    fn builder_wrapper_type(&self, fctx: &FXFieldCtx, turbo_fish: bool) -> darling::Result<TokenStream> {
        let ty = fctx.ty();
        let fallible = fctx.fallible();
        let span = fallible.or(fctx.builder()).final_span();
        let input_type = self.input_type_toks();
        let (wrapper_type, error_type) = if *fallible {
            let error_type = fctx.fallible_error();
            let fallible_span = fallible.final_span();
            (
                self.implementor(fctx).fx_fallible_builder_wrapper(span),
                quote_spanned![fallible_span=> , #error_type],
            )
        }
        else {
            (self.implementor(fctx).fx_infallible_builder_wrapper(span), quote![])
        };

        let dcolon = if turbo_fish {
            quote_spanned![span=> ::]
        }
        else {
            quote![]
        };

        Ok(quote_spanned! {span=> #wrapper_type #dcolon <#input_type, #ty #error_type>})
    }

    fn wrap_builder(&self, fctx: &FXFieldCtx, builder: TokenStream) -> darling::Result<TokenStream> {
        let wrapper_type = self.builder_wrapper_type(fctx, true)?;
        let span = fctx.fallible().final_span();
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
            let ty = fctx.ty().to_token_stream();

            if *fctx.skipped() {
                return ty;
            }

            let lazy = fctx.lazy();
            if *lazy {
                let proxy_type = self.implementor(fctx).field_proxy_type(lazy.final_span());
                let span = fctx.ty().span();
                quote_spanned! [span=> #proxy_type<#builder_wrapper_type>]
            }
            else {
                self.maybe_locked_ty(fctx, &self.maybe_optional_ty(fctx, &ty))
            }
        }))
    }

    fn ref_count_types(&self, span: Span) -> (TokenStream, TokenStream) {
        (
            quote_spanned![span=> ::std::sync::Arc],
            quote_spanned![span=> ::std::sync::Weak],
        )
    }

    fn field_lazy_builder_wrapper(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if *fctx.lazy() {
            self.implementor(fctx).lazy_wrapper_fn(self, fctx)
        }
        else {
            Ok(quote![])
        }
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let accessor = fctx.accessor();
        Ok(if *accessor {
            let span = accessor.final_span();

            let accessor_mode = fctx.accessor_mode();
            let is_copy = accessor_mode.is_copy();
            let is_clone = accessor_mode.is_clone();
            let lazy = fctx.lazy();
            let lock = fctx.lock();

            if !(is_copy || is_clone) && (*lazy || *lock) {
                return self.field_reader_method(
                    fctx,
                    fctx.accessor_ident(),
                    fctx.accessor_visibility(),
                    fctx.helper_attributes_fn(FXHelperKind::Accessor, FXInlining::Always, span),
                    // It's preferable for the reader method's span to be bound to the arguments that request it,
                    // prioritizing lock-related attributes over lazy ones, since the latter implicitly triggers method
                    // generation whereas the former has direct relation to it.
                    lock.or(lazy).final_span(),
                );
            }

            let mut mc = MethodConstructor::new(fctx.accessor_ident());
            let is_optional = *fctx.optional();
            let ident = fctx.ident();
            let ty = fctx.ty();
            let read_method = self.read_method_name(fctx, false, span);

            self.maybe_ref_counted_self(fctx, &mut mc);
            mc.set_span(span);
            mc.set_vis(fctx.accessor_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Accessor, FXInlining::Always, span));

            if is_clone || is_copy {
                let cc_span = fctx.accessor_mode().final_span();
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

                if *lazy {
                    let implementor = self.implementor(fctx);
                    let await_call = implementor.await_call(span);
                    let self_rc = mc.self_maybe_rc();
                    let ty = self.fallible_return_type(fctx, ty)?;

                    mc.set_async(fctx.mode_async());
                    mc.set_ret_type(ty);
                    mc.add_statement(
                        quote_spanned! {span=> let rlock = self.#ident.#read_method(&#self_rc) #await_call #shortcut; },
                    );
                    mc.set_ret_stmt(fctx.fallible_ok_return(&quote_spanned! {span=> (*rlock)#cmethod}));
                }
                else if *fctx.lock() {
                    let lock_span = fctx.lock().final_span();
                    mc.set_ret_type(self.maybe_optional_ty(fctx, ty));
                    mc.add_statement(quote_spanned! {lock_span=> let rlock = self.#ident.read() #shortcut; });
                    mc.set_ret_stmt(quote_spanned! {lock_span=> (*rlock)#cmethod});
                }
                else if is_optional {
                    mc.set_ret_type(self.maybe_optional_ty(fctx, ty));
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident #cmethod });
                }
                else {
                    let cmethod = if is_copy {
                        quote![]
                    }
                    else {
                        quote_spanned![accessor_mode.final_span()=> .clone()]
                    };
                    mc.set_ret_type(ty.to_token_stream());
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident #cmethod });
                }
            }
            else {
                let ty = self.fallible_return_type(fctx, &self.maybe_optional_ty(fctx, ty))?;

                mc.set_ret_type(quote_spanned! {span=> &#ty });
                mc.set_ret_stmt(quote_spanned! {span=> &self.#ident });
            }

            mc.to_method()
        }
        else {
            quote![]
        })
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let accessor_mut = fctx.accessor_mut();
        Ok(if *accessor_mut {
            let mut mc = MethodConstructor::new(fctx.accessor_mut_ident());
            let span = accessor_mut.final_span();
            let ident = fctx.ident();
            let ty = fctx.ty();
            let read_method = self.read_method_name(fctx, true, span);

            self.maybe_ref_counted_self(fctx, &mut mc);
            mc.set_span(span);
            mc.set_vis(fctx.accessor_mut_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::AccessorMut, FXInlining::Always, span));

            let lazy = fctx.lazy();

            if *lazy {
                let lazy_span = lazy.final_span();
                let implementor = self.implementor(fctx);
                let self_rc = mc.self_maybe_rc();
                let await_call = implementor.await_call(span);
                let mapped_guard = self.implementor(fctx).rwlock_mapped_write_guard(lazy_span);
                let lifetime = quote_spanned! {lazy_span=> 'fx_get_mut};

                mc.set_async(fctx.mode_async());
                mc.set_self_lifetime(lifetime.clone());
                mc.set_ret_type(
                    self.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<#lifetime, #ty> })?,
                );
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method(&#self_rc) #await_call });
            }
            else {
                let optional = fctx.optional();
                let lock = fctx.lock();

                let ty_toks = if *optional {
                    quote_spanned![optional.final_span()=> ::std::option::Option<#ty>]
                }
                else {
                    ty.to_token_stream()
                };

                if *lock {
                    let lock_span = lock.final_span();
                    let wrguard = self.implementor(fctx).rwlock_write_guard(lock_span);

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

            mc.to_method()
        }
        else {
            quote![]
        })
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let builder = fctx.forced_builder().or(fctx.builder());
        let span = builder.final_span();
        let field_ident = fctx.ident();
        let field_type = self.type_tokens(fctx)?;
        let optional = fctx.optional();
        let lazy = fctx.lazy();
        let default_value = self.field_default_value(fctx);
        let lazy_builder = self.implementor(fctx).lazy_builder(self, fctx);
        let or_default = if fctx.has_default_value() {
            let default = self.fixup_self_type(default_value.clone().expect(&format!(
                "Internal problem: expected default value for field {}",
                field_ident
            )));

            let opt_or_lazy = optional.or(lazy);
            if *opt_or_lazy {
                quote_spanned! [opt_or_lazy.final_span()=> .or_else(|| ::std::option::Option::Some(#default))]
            }
            else {
                quote_spanned! [span=> .unwrap_or_else(|| #default)]
            }
        }
        else {
            quote![]
        };

        Ok(if !*builder {
            // Though this branch may look weird, it makes sense if the builder struct is requested implicitly by
            // using a `builder` argument with a field attribute.
            let default = self.field_value_wrap(fctx, default_value)?;
            quote_spanned![span=> #field_ident: #default]
        }
        else if *lazy {
            let lazy_builder = self.wrap_builder(fctx, lazy_builder)?;
            quote_spanned![span=>
                #field_ident: <#field_type>::new_default(
                    #lazy_builder,
                    self.#field_ident.take()#or_default
                )
            ]
        }
        else if *fctx.lock() {
            let set_value = self.field_builder_value_for_set(fctx, field_ident, &span);
            quote_spanned![span=>
                // Optional can simply pickup and own either Some() or None from the builder.
                #field_ident: #set_value
            ]
        }
        else if *optional {
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
        let field_ident = fctx.ident();
        let shadow_var = self.ctx().shadow_var_ident();
        self.field_value_wrap(fctx, FXValueRepr::Exact(quote![#shadow_var.#field_ident ]))
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident();
        let me_var = self.ctx().me_var_ident();
        let mut field_access = quote_spanned! {field_ident.span()=> #me_var.#field_ident };
        let into_inner = fctx.serde_optional().or(fctx.lock());
        if *into_inner {
            field_access = quote_spanned! {into_inner.final_span()=> #field_access.into_inner() };
        }
        Ok(field_access)
    }

    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let reader = fctx.reader();
        if *reader {
            let span = reader.final_span();
            self.field_reader_method(
                fctx,
                fctx.reader_ident(),
                fctx.reader_visibility(),
                fctx.helper_attributes_fn(FXHelperKind::Reader, FXInlining::Always, span),
                span,
            )
        }
        else {
            Ok(quote![])
        }
    }

    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let writer = fctx.writer();
        Ok(if *writer {
            let span = writer.final_span();
            let mut mc = MethodConstructor::new(fctx.writer_ident());
            let ident = fctx.ident();
            let mut ret_ty = fctx.ty().to_token_stream();
            let await_call = self.implementor(fctx).await_call(span);
            let implementor = self.implementor(fctx);

            mc.set_vis(fctx.writer_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Writer, FXInlining::Always, span));
            mc.set_async(fctx.mode_async());

            let lazy = fctx.lazy();

            if *lazy {
                let lazy_span = lazy.final_span();
                let lifetime = quote_spanned! {lazy_span=> 'fx_writer_lifetime};
                let fx_wrlock_guard = implementor.fx_mapped_write_guard(lazy_span);
                let builder_wrapper_type = self.builder_wrapper_type(fctx, false)?;

                mc.set_self_lifetime(lifetime.clone());
                mc.set_ret_type(quote_spanned! {span=> #fx_wrlock_guard<#lifetime, #builder_wrapper_type>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call});
            }
            else {
                let wrlock_guard = implementor.rwlock_write_guard(span);
                let optional = fctx.optional();

                if *optional {
                    ret_ty = quote_spanned! {optional.final_span()=> ::std::option::Option<#ret_ty>};
                }

                mc.set_ret_type(quote_spanned! {span=> #wrlock_guard<#ret_ty>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call});
            }

            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let setter = fctx.setter();
        Ok(if *setter {
            let span = setter.final_span();
            let implementor = self.implementor(fctx);
            let mut mc = MethodConstructor::new(fctx.setter_ident());
            let ident = fctx.ident();
            let ty = fctx.ty();
            let (val_type, gen_params, into_tok) = self.into_toks(fctx, fctx.setter_into());
            let await_call = implementor.await_call(span);
            let value_tok = quote_spanned! {span=> value #into_tok};
            let lazy = fctx.lazy();
            let optional = fctx.optional();
            let lock = fctx.lock();

            mc.set_span(span);
            mc.set_vis(fctx.setter_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Setter, FXInlining::Always, span));
            mc.maybe_add_generic(gen_params);
            mc.add_param(quote_spanned! {span=> value: #val_type});

            if *lazy || *optional || *lock {
                mc.set_async(fctx.mode_async());
            }

            if *lazy {
                let lazy_span = lazy.final_span();
                mc.set_ret_type(quote_spanned! {lazy_span=> ::std::option::Option<#ty>});
                mc.set_ret_stmt(quote_spanned! {lazy_span=> self.#ident.write()#await_call.store(#value_tok)});
            }
            else if *optional {
                let opt_span = optional.final_span();
                let (lock_method, opt_await_call) = if *lock {
                    let lock_span = lock.final_span();
                    (quote_spanned! {lock_span=> .write()}, await_call)
                }
                else {
                    mc.set_self_mut(true);

                    (quote![], quote![])
                };

                mc.set_ret_type(quote_spanned! {opt_span=> ::std::option::Option<#ty>});
                mc.set_ret_stmt(
                    quote_spanned! {opt_span=> self.#ident #lock_method #opt_await_call.replace(#value_tok)},
                );
            }
            else if *lock {
                let lock_span = lock.final_span();
                mc.set_ret_type(ty.to_token_stream());
                mc.add_statement(quote_spanned! {lock_span=> let mut wlock = self.#ident.write()#await_call; });
                mc.set_ret_stmt(quote_spanned! {lock_span=> ::std::mem::replace(&mut *wlock, #value_tok)});
            }
            else {
                mc.set_ret_type(ty.to_token_stream());
                mc.set_self_mut(true);
                mc.set_ret_stmt(quote_spanned! {span=> ::std::mem::replace(&mut self.#ident, #value_tok)});
            }

            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let clearer = fctx.clearer();
        Ok(if *clearer {
            let span = clearer.final_span();
            let mut mc = MethodConstructor::new(fctx.clearer_ident());
            let ident = fctx.ident();
            let ty = fctx.ty();
            let implementor = self.implementor(fctx);
            let await_call = implementor.await_call(span);
            let lock = fctx.lock();

            mc.set_vis(fctx.clearer_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Clearer, FXInlining::Always, span));
            mc.set_async(fctx.mode_async());
            mc.set_ret_type(quote_spanned! {span=> ::std::option::Option<#ty>});

            let lazy = fctx.lazy();

            if *lazy {
                mc.set_ret_stmt(quote_spanned! {lazy.final_span()=> self.#ident.clear()#await_call});
            }
            else if *lock {
                // If not lazy then it's optional
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call.take()});
            }
            else {
                mc.set_self_mut(true);
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident #await_call.take()});
            }

            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let predicate = fctx.predicate();
        Ok(if *predicate {
            let span = predicate.final_span();
            let mut mc = MethodConstructor::new(fctx.predicate_ident());
            let ident = fctx.ident();

            mc.set_vis(fctx.predicate_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Predicate, FXInlining::Always, span));
            mc.set_ret_type(quote_spanned! {span=> bool});

            let lazy = fctx.lazy();
            let lock = fctx.lock();

            if *lazy {
                mc.set_ret_stmt(quote_spanned! {lazy.final_span()=> self.#ident.is_set()});
            }
            // If not lazy then it's optional
            else if *lock {
                let lock_span = lock.final_span();
                mc.set_async(fctx.mode_async());
                let await_call = self.implementor(fctx).await_call(lock_span);
                let read_method = self.read_method_name(fctx, false, lock_span);
                mc.set_ret_stmt(quote_spanned! {lock_span=> self.#ident.#read_method()#await_call.is_some()});
            }
            else {
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.is_some()});
            }

            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: FXValueRepr<TokenStream>) -> darling::Result<TokenStream> {
        let ident_span = fctx.ident().span();
        let optional = fctx.optional();
        let lazy = fctx.lazy();

        let value_wrapper = match &value {
            FXValueRepr::None => quote_spanned![ident_span=> ::std::option::Option::None],
            FXValueRepr::Exact(v) => quote_spanned![ident_span=> #v],
            FXValueRepr::Versatile(v) => {
                if *optional || *lazy {
                    quote_spanned![optional.or(lazy).final_span()=> ::std::option::Option::Some(#v)]
                }
                else {
                    quote_spanned![ident_span=> #v]
                }
            }
        };

        Ok(if *lazy {
            let field_type = self.type_tokens(fctx)?;
            let lazy_builder = self.wrap_builder(fctx, self.implementor(fctx).lazy_builder(self, fctx))?;

            quote_spanned![lazy.final_span()=> <#field_type>::new_default(#lazy_builder, #value_wrapper) ]
        }
        else {
            if !*optional && value.is_none() {
                return Err(darling::Error::custom(format!(
                    "No value was supplied for non-optional, non-lazy field '{}'",
                    fctx.ident()
                )));
            }

            let lock = fctx.lock();
            if *lock {
                let lock_span = lock.final_span();
                let rwlock = self.implementor(fctx).rwlock(lock_span);
                quote_spanned![lock_span=> #rwlock::new(#value_wrapper) ]
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
