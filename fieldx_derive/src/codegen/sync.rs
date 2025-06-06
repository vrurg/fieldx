use fieldx_aux::FXPropBool;
use fieldx_core::codegen::constructor::FXConstructor;
use fieldx_core::codegen::constructor::FXFnConstructor;
use fieldx_core::ctx::FXCodeGenCtx;
use fieldx_core::types::helper::FXHelperKind;
use fieldx_core::types::meta::FXToksMeta;
use fieldx_core::types::meta::FXValueFlag;
use fieldx_core::types::FXInlining;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use std::rc::Rc;
use syn::spanned::Spanned;

use crate::codegen::codegen_trait::FXAccessorElements;

use super::derive_ctx::FXDeriveCodegenCtx;
use super::derive_ctx::FXDeriveFieldCtx;
use super::derive_ctx::FXDeriveMacroCtx;
use super::FXCodeGenContextual;
use super::FXValueRepr;

pub(crate) struct FXCodeGenSync<'a> {
    codegen: &'a crate::codegen::FXRewriter<'a>,
    ctx:     Rc<FXCodeGenCtx<FXDeriveMacroCtx>>,
}

impl<'a> FXCodeGenSync<'a> {
    pub(crate) fn new(codegen: &'a crate::codegen::FXRewriter<'a>, ctx: Rc<FXCodeGenCtx<FXDeriveMacroCtx>>) -> Self {
        Self { codegen, ctx }
    }

    fn read_method_name(&self, fctx: &FXDeriveFieldCtx, mutable: bool, span: Span) -> syn::Ident {
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
        fctx: &FXDeriveFieldCtx,
        helper_ident: &syn::Ident,
        helper_vis: &syn::Visibility,
        attributes_fn: TokenStream,
        no_lifetime: bool,
        span: Span,
    ) -> darling::Result<FXFnConstructor> {
        let mut helper_ident = helper_ident.clone();
        helper_ident.set_span(span);
        let mut mc = FXFnConstructor::new(helper_ident);
        let ident = fctx.ident();
        let ty = fctx.ty();
        let implementor = fctx.impl_details();
        let rwlock_guard = implementor.rwlock_read_guard(span)?;
        let await_call = implementor.await_call(span);
        let read_method = self.read_method_name(fctx, false, span);
        let lifetime = if no_lifetime {
            quote![]
        }
        else {
            quote_spanned! {span=> 'fx_reader_lifetime}
        };

        mc.set_vis(helper_vis.clone())
            .set_span(span)
            .set_async(fctx.mode_async())
            .add_attribute_toks(attributes_fn)?;

        let lazy = fctx.lazy();
        let optional = fctx.optional();

        // Tokens for set_ret_type and set_ret_stmt are generated using the default span because the components of the
        // return type that relate to specific arguments (lazy, optional) are already bound to their respective spans.
        // However, the surrounding syntax belongs to the method itself.
        if *lazy {
            self.maybe_ref_counted_self(fctx, &mut mc)?;
            let lazy_span = lazy.final_span();
            let mapped_guard = implementor.rwlock_mapped_read_guard(lazy_span)?;
            let self_rc = mc.self_maybe_rc_as_ref()
                .ok_or(
                    darling::Error::custom("Missing information about the `self` identifier, but reader method cannot be an associated function")
                        .with_span(&span),
                )?;

            mc.set_self_lifetime(lifetime.clone());
            mc.set_ret_type(fctx.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<#lifetime, #ty> })?);
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method(#self_rc)#await_call});
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
            mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method()#await_call});
        }

        Ok(mc)
    }

    #[inline(always)]
    fn maybe_locked_ty<T: ToTokens>(&self, fctx: &FXDeriveFieldCtx, ty: &T) -> darling::Result<TokenStream> {
        let lock = fctx.lock();
        Ok(if *lock {
            let span = lock.final_span();
            let rwlock = fctx.impl_details().rwlock(span)?;
            quote_spanned![span=> #rwlock<#ty>]
        }
        else {
            ty.to_token_stream()
        })
    }

    // Compose declaration of the type to use for holding the builder method object.
    fn builder_wrapper_type(&self, fctx: &FXDeriveFieldCtx, turbo_fish: bool) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        let ty = fctx.ty();
        if *fctx.lock() {
            let fallible = fctx.fallible();
            let span = fallible.or(fctx.builder()).final_span();
            let input_type = ctx.struct_type_toks();
            let implementor = fctx.impl_details();
            let (wrapper_type, error_type) = if *fallible {
                let error_type = fctx.fallible_error();
                let fallible_span = fallible.final_span();
                (
                    implementor.fx_fallible_builder_wrapper(span)?,
                    quote_spanned![fallible_span=> , #error_type],
                )
            }
            else {
                (implementor.fx_infallible_builder_wrapper(span)?, quote![])
            };

            let dcolon = if turbo_fish {
                quote_spanned![span=> ::]
            }
            else {
                quote![]
            };

            Ok(quote_spanned! {span=> #wrapper_type #dcolon <#input_type, #ty #error_type>})
        }
        else {
            Ok(ty.to_token_stream())
        }
    }

    fn wrap_builder(&self, fctx: &FXDeriveFieldCtx, builder: TokenStream) -> darling::Result<TokenStream> {
        if *fctx.lock() {
            let wrapper_type = self.builder_wrapper_type(fctx, true)?;
            let span = fctx.fallible().final_span();
            Ok(quote_spanned! {span=>
                #wrapper_type::new(#builder)
            })
        }
        else {
            Ok(builder)
        }
    }

    fn new_lazy_field_container(
        &self,
        fctx: &FXDeriveFieldCtx,
        lazy_builder: TokenStream,
        value: TokenStream,
        span: Span,
    ) -> darling::Result<TokenStream> {
        let field_type = self.type_tokens(fctx)?;
        if *fctx.lock() {
            Ok(quote_spanned! {span=> <#field_type>::new_default(#lazy_builder, #value)})
        }
        else {
            let module = fctx.impl_details().fieldx_impl_mod(span);
            Ok(quote_spanned! {span=> #module::new_lazy_container(#value)})
        }
    }

    fn field_proxy_type(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<TokenStream> {
        Ok(if *fctx.lazy() {
            let implementor = fctx.impl_details();
            if *fctx.lock() {
                implementor.field_lock_proxy_type(fctx.lazy().final_span())?
            }
            else {
                implementor.field_simple_proxy_type(fctx.lazy().final_span())
            }
        }
        else {
            quote![]
        })
    }
}

impl<'a> FXCodeGenContextual for FXCodeGenSync<'a> {
    #[inline(always)]
    fn ctx(&self) -> &Rc<FXDeriveCodegenCtx> {
        &self.ctx
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXDeriveFieldCtx) -> darling::Result<&'s TokenStream> {
        let builder_wrapper_type = self.builder_wrapper_type(fctx, false)?;
        fctx.ty_wrapped(|| {
            let ty = fctx.ty().to_token_stream();

            if *fctx.skipped() {
                return Ok(ty);
            }

            let lazy = fctx.lazy();
            if *lazy {
                let proxy_type = self.field_proxy_type(fctx)?;
                let span = fctx.ty().span();
                Ok(quote_spanned! [span=> #proxy_type<#builder_wrapper_type>])
            }
            else {
                self.maybe_locked_ty(fctx, &self.maybe_optional(fctx, &ty))
            }
        })
    }

    fn field_lazy_builder_wrapper(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        if *fctx.lazy() && *fctx.lock() {
            fctx.impl_details().lazy_wrapper_fn(fctx)
        }
        else {
            Ok(None)
        }
    }

    fn field_accessor(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let accessor = fctx.accessor();
        Ok(if *accessor {
            let span = accessor.final_span();

            let accessor_mode = fctx.accessor_mode();
            let is_copy = accessor_mode.is_copy();
            let is_clone = accessor_mode.is_clone();
            let lazy = fctx.lazy();
            let lock = fctx.lock();

            let FXAccessorElements {
                reference,
                method,
                type_ref,
                dereference,
            } = self.accessor_elements(fctx);

            let mut mc = if *lock {
                self.field_reader_method(
                    fctx,
                    fctx.accessor_ident(),
                    fctx.accessor_visibility(),
                    fctx.helper_attributes_fn(FXHelperKind::Accessor, FXInlining::Always, span),
                    is_copy || is_clone,
                    lock.final_span(),
                )?
            }
            else {
                let mut mc = FXFnConstructor::new(fctx.accessor_ident().clone());
                mc.set_span(span)
                    .set_vis(fctx.accessor_visibility())
                    .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Accessor, FXInlining::Always, span))?;
                mc
            };

            let ident = fctx.ident();
            let shortcut = fctx.fallible_shortcut();
            let implementor = fctx.impl_details();
            let await_call = implementor.await_call(span);
            let ty = fctx.ty();
            let ty = self.maybe_optional(fctx, &quote_spanned! {span=> #type_ref #ty });
            let ty = fctx.fallible_return_type(fctx, &quote_spanned! {span => #reference #ty})?;

            if *lazy && !*lock {
                let lazy_span = lazy.final_span();
                self.maybe_ref_counted_self(fctx, &mut mc)?;

                let lazy_init = self.field_lazy_initializer(fctx, &mut mc)?;

                mc.set_ret_type(ty);
                mc.set_async(fctx.mode_async());
                if accessor_mode.is_none() {
                    // When no accessor mode is set by user there is no need to wrap the return expression into Ok()
                    // for fallibles because the return error type is what the builder method would return.
                    mc.set_ret_stmt(quote_spanned! {lazy_span=>
                        self.#ident #lazy_init #await_call
                    });
                }
                else {
                    mc.set_ret_stmt(fctx.fallible_ok_return(&quote_spanned! {lazy_span=>
                        #dereference ( self.#ident #lazy_init #await_call #shortcut ) #method
                    }));
                }
            }
            else if *lock {
                if is_clone || is_copy {
                    mc.set_ret_type(ty.clone());
                    let ret_stmt = mc.ret_stmt();
                    mc.set_ret_stmt(quote_spanned! {ret_stmt.span()=> #dereference #ret_stmt #method });
                }
            }
            else {
                mc.set_ret_type(quote_spanned! {span=> #ty });
                mc.set_ret_stmt(quote_spanned! {span=> #reference self.#ident #method });
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_accessor_mut(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let accessor_mut = fctx.accessor_mut();
        Ok(if *accessor_mut {
            let mut mc = FXFnConstructor::new(fctx.accessor_mut_ident().clone());
            let span = accessor_mut.final_span();
            let ident = fctx.ident();
            let ty = fctx.ty();
            let read_method = self.read_method_name(fctx, true, span);
            let lock = fctx.lock();

            mc.set_span(span)
                .set_vis(fctx.accessor_mut_visibility())
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::AccessorMut, FXInlining::Always, span))?;

            let lazy = fctx.lazy();
            let implementor = fctx.impl_details();
            let await_call = implementor.await_call(span);

            if *lazy {
                self.maybe_ref_counted_self(fctx, &mut mc)?;
                let lazy_span = lazy.final_span();

                mc.set_async(fctx.mode_async());

                if *lock {
                    let self_rc = mc.self_maybe_rc_as_ref()
                    .ok_or(
                        darling::Error::custom("Missing information about the `self` identifier, but mutable accessor method cannot be an associated function")
                            .with_span(&span),
                    )?;
                    let mapped_guard = implementor.rwlock_mapped_write_guard(lazy_span)?;
                    let lifetime = quote_spanned! {lazy_span=> 'fx_get_mut};

                    mc.set_self_lifetime(lifetime.clone());
                    mc.set_ret_type(
                        fctx.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<#lifetime, #ty> })?,
                    );
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident.#read_method(#self_rc) #await_call });
                }
                else {
                    let lazy_init = self.field_simple_lazy_initializer(fctx, &mut mc)?;
                    let shortcut = fctx.fallible_shortcut();
                    let self_ident = mc.self_ident();

                    mc.add_statement(quote_spanned! {
                        span=> #self_ident.#ident #lazy_init #await_call #shortcut;
                    })
                    .set_self_mut(true)
                    .set_ret_type(fctx.fallible_return_type(fctx, quote_spanned! {lazy_span=> &mut #ty})?)
                    .set_ret_stmt(
                        fctx.fallible_ok_return(&quote_spanned! {lazy_span=> #self_ident.#ident .get_mut().unwrap()}),
                    );
                }
            }
            else {
                let optional = fctx.optional();

                let ty_toks = if *optional {
                    quote_spanned![optional.final_span()=> ::std::option::Option<#ty>]
                }
                else {
                    ty.to_token_stream()
                };

                if *lock {
                    let lock_span = lock.final_span();
                    let wrguard = implementor.rwlock_write_guard(lock_span)?;
                    let lifetime = quote_spanned! {lock_span=> 'fx_mut_lifetime};

                    mc.set_async(fctx.mode_async())
                        .set_self_lifetime(lifetime.clone())
                        .set_ret_type(quote_spanned! [lock_span=> #wrguard<#lifetime, #ty_toks>])
                        .set_ret_stmt(quote_spanned! [lock_span=> self.#ident.write() #await_call]);
                }
                else {
                    // Bare field
                    return self.codegen.plain_gen().field_accessor_mut(fctx);
                    // mc.set_ret_type(quote_spanned! {span=> &mut #ty});
                    // mc.set_ret_stmt(quote_spanned! {span=> &mut self.#ident });
                }
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_builder_setter(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<TokenStream> {
        let builder = fctx.forced_builder().or(fctx.builder());
        let span = builder.final_span();
        let field_ident = fctx.ident();
        let optional = fctx.optional();
        let lazy = fctx.lazy();
        let lock = *fctx.lock();
        let default_value = self.field_default_value(fctx);
        let lazy_builder = fctx.impl_details().lazy_builder(fctx);
        let or_default = if fctx.has_default_value() {
            let default = self.fixup_self_type(
                default_value
                    .clone()
                    .expect(&format!(
                        "Internal problem: expected default value for field {field_ident}"
                    ))
                    .into_inner(),
            );

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
            let attributes = default.attributes.clone();
            quote_spanned![span=>
                #( #attributes )*
                #field_ident: #default
            ]
        }
        else if *lazy {
            let lazy_builder = self.wrap_builder(fctx, lazy_builder)?;
            let init = self.new_lazy_field_container(
                fctx,
                lazy_builder,
                quote_spanned! {span=> self.#field_ident.take()#or_default},
                span,
            )?;
            quote_spanned! {span=> #field_ident: #init}
        }
        else if lock {
            let set_value = self.field_builder_value_for_set(fctx, field_ident, &span);
            let attributes = set_value.attributes.clone();
            quote_spanned![span=>
                // Optional can simply pickup and own either Some() or None from the builder.
                #( #attributes )*
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

    fn field_lazy_initializer(
        &self,
        fctx: &FXDeriveFieldCtx,
        mc: &mut FXFnConstructor,
    ) -> darling::Result<TokenStream> {
        if *fctx.lock() {
            let self_var = mc.self_maybe_rc();
            let span = fctx.lazy().final_span();
            Ok(quote_spanned! {span=> .lazy_init(&#self_var)})
        }
        else {
            self.field_simple_lazy_initializer(fctx, mc)
        }
    }

    #[cfg(feature = "serde")]
    fn field_from_shadow(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta> {
        let field_ident = fctx.ident();
        let impl_ctx = self.ctx().impl_ctx();
        let shadow_var = impl_ctx.shadow_var_ident()?;
        self.field_value_wrap(
            fctx,
            FXValueRepr::Exact(quote_spanned![shadow_var.span()=> #shadow_var.#field_ident ].into()),
        )
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta> {
        let field_ident = fctx.ident();
        let impl_ctx = self.ctx().impl_ctx();
        let me_var = impl_ctx.me_var_ident()?;
        let mut field_access = quote_spanned! {field_ident.span()=> #me_var.#field_ident };
        let into_inner = fctx.lock().or(fctx.lazy());
        if *into_inner {
            field_access = quote_spanned! {into_inner.final_span()=> #field_access.into_inner() };
        }
        Ok(field_access.into())
    }

    fn field_reader(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let reader = fctx.reader();
        if *reader {
            let span = reader.final_span();
            self.field_reader_method(
                fctx,
                fctx.reader_ident(),
                fctx.reader_visibility(),
                fctx.helper_attributes_fn(FXHelperKind::Reader, FXInlining::Always, span),
                false,
                span,
            )
            .map(Some)
        }
        else {
            Ok(None)
        }
    }

    fn field_writer(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let writer = fctx.writer();
        Ok(if *writer {
            let span = writer.final_span();
            let mut mc = FXFnConstructor::new(fctx.writer_ident().clone());
            let ident = fctx.ident();
            let mut ret_ty = fctx.ty().to_token_stream();
            let implementor = fctx.impl_details();
            let await_call = implementor.await_call(span);

            mc.set_vis(fctx.writer_visibility())
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Writer, FXInlining::Always, span))?
                .set_async(fctx.mode_async());

            let lazy = fctx.lazy();

            if *lazy {
                let lazy_span = lazy.final_span();
                let lifetime = quote_spanned! {lazy_span=> 'fx_writer_lifetime};
                let fx_wrlock_guard = implementor.fx_mapped_write_guard(lazy_span)?;
                let builder_wrapper_type = self.builder_wrapper_type(fctx, false)?;

                mc.set_self_lifetime(lifetime.clone());
                mc.set_ret_type(quote_spanned! {span=> #fx_wrlock_guard<#lifetime, #builder_wrapper_type>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call});
            }
            else {
                let wrlock_guard = implementor.rwlock_write_guard(span)?;
                let optional = fctx.optional();

                if *optional {
                    ret_ty = quote_spanned! {optional.final_span()=> ::std::option::Option<#ret_ty>};
                }

                mc.set_ret_type(quote_spanned! {span=> #wrlock_guard<#ret_ty>});
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call});
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_setter(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let setter = fctx.setter();
        Ok(if *setter {
            let span = setter.final_span();
            let implementor = fctx.impl_details();
            let mut mc = FXFnConstructor::new(fctx.setter_ident().clone());
            let ident = fctx.ident();
            let ty = fctx.ty();
            let (val_type, gen_params, into_tok) = self.to_toks(fctx, fctx.setter_into());
            let await_call = implementor.await_call(span);
            let value_toks = quote_spanned! {span=> value #into_tok};
            let lazy = fctx.lazy();
            let optional = fctx.optional();
            let lock = fctx.lock();

            mc.set_span(span)
                .set_vis(fctx.setter_visibility())
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Setter, FXInlining::Always, span))?
                .maybe_add_generic(gen_params)
                .add_param(quote_spanned! {span=> value: #val_type});

            if *lazy || *optional || *lock {
                mc.set_async(fctx.mode_async());
            }

            if *lazy {
                let lazy_span = lazy.final_span();
                if *lock {
                    mc.set_ret_type(quote_spanned! {lazy_span=> ::std::option::Option<#ty>});
                    mc.set_ret_stmt(quote_spanned! {lazy_span=> self.#ident.write()#await_call.store(#value_toks)});
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_type(quote_spanned! {lazy_span=> ::std::option::Option<#ty>});
                    mc.add_statement(quote_spanned! {span=>
                        let old = self.#ident.take();
                        let _ = self.#ident.set(#value_toks);
                    });
                    mc.set_ret_stmt(quote_spanned! {span=> old});
                }
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
                    quote_spanned! {opt_span=> self.#ident #lock_method #opt_await_call.replace(#value_toks)},
                );
            }
            else if *lock {
                let lock_span = lock.final_span();
                mc.set_ret_type(ty.to_token_stream());
                mc.add_statement(quote_spanned! {lock_span=> let mut wlock = self.#ident.write()#await_call; });
                mc.set_ret_stmt(quote_spanned! {lock_span=> ::std::mem::replace(&mut *wlock, #value_toks)});
            }
            else {
                mc.set_ret_type(ty.to_token_stream());
                mc.set_self_mut(true);
                mc.set_ret_stmt(quote_spanned! {span=> ::std::mem::replace(&mut self.#ident, #value_toks)});
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_clearer(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let clearer = fctx.clearer();
        Ok(if *clearer {
            let span = clearer.final_span();
            let mut mc = FXFnConstructor::new(fctx.clearer_ident().clone());
            let ident = fctx.ident();
            let ty = fctx.ty();
            let implementor = fctx.impl_details();
            let await_call = implementor.await_call(span);
            let lock = fctx.lock();

            mc.set_vis(fctx.clearer_visibility())
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Clearer, FXInlining::Always, span))?
                .set_async(fctx.mode_async())
                .set_ret_type(quote_spanned! {span=> ::std::option::Option<#ty>});

            let lazy = fctx.lazy();

            if *lazy {
                if *lock {
                    mc.set_ret_stmt(quote_spanned! {lazy.final_span()=> self.#ident.clear()#await_call});
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_stmt(quote_spanned! {lazy.final_span()=> self.#ident.take()});
                }
            }
            else if *lock {
                // If not lazy then it's optional
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.write()#await_call.take()});
            }
            else {
                mc.set_self_mut(true);
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident #await_call.take()});
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_predicate(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let predicate = fctx.predicate();
        Ok(if *predicate {
            let span = predicate.final_span();
            let mut mc = FXFnConstructor::new(fctx.predicate_ident().clone());
            let ident = fctx.ident();

            mc.set_vis(fctx.predicate_visibility())
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Predicate, FXInlining::Always, span))?
                .set_ret_type(quote_spanned! {span=> bool});

            let lazy = fctx.lazy();
            let lock = fctx.lock();

            if *lazy {
                mc.set_ret_stmt(quote_spanned! {lazy.final_span()=> self.#ident.is_set()});
            }
            // If not lazy then it's optional
            else if *lock {
                let lock_span = lock.final_span();
                mc.set_async(fctx.mode_async());
                let await_call = fctx.impl_details().await_call(lock_span);
                let read_method = self.read_method_name(fctx, false, lock_span);
                mc.set_ret_stmt(quote_spanned! {lock_span=> self.#ident.#read_method()#await_call.is_some()});
            }
            else {
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.is_some()});
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_value_wrap(&self, fctx: &FXDeriveFieldCtx, value: FXValueRepr<FXToksMeta>) -> darling::Result<FXToksMeta> {
        let ident_span = fctx.ident().span();
        let optional = fctx.optional();
        let lazy = fctx.lazy();

        let value_wrapper = match value.clone() {
            FXValueRepr::None => quote_spanned![ident_span=> ::std::option::Option::None].into(),
            FXValueRepr::Exact(v) => v,
            FXValueRepr::Versatile(v) => {
                if *optional || *lazy {
                    let value_toks = v.to_token_stream();
                    v.replace(quote_spanned![optional.or(lazy).final_span()=> ::std::option::Option::Some(#value_toks)])
                        .mark_as(FXValueFlag::ContainerWrapped)
                }
                else {
                    v
                }
            }
        };

        Ok(if *lazy {
            let lazy_builder = self.wrap_builder(fctx, fctx.impl_details().lazy_builder(fctx))?;
            let value_toks = value_wrapper.to_token_stream();

            value_wrapper
                .replace(self.new_lazy_field_container(fctx, lazy_builder, value_toks, lazy.final_span())?)
                .mark_as(FXValueFlag::ContainerWrapped)
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
                let rwlock = fctx.impl_details().rwlock(lock_span)?;
                let value_toks = value_wrapper.to_token_stream();
                value_wrapper
                    .replace(quote_spanned![lock_span=> #rwlock::new(#value_toks) ])
                    .mark_as(FXValueFlag::ContainerWrapped)
            }
            else {
                value_wrapper
            }
        })
    }

    fn field_default_wrap(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta> {
        self.field_value_wrap(fctx, self.field_default_value(fctx))
    }
}
