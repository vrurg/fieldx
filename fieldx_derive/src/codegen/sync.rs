mod impl_async;
mod impl_sync;

#[cfg(feature = "serde")]
use crate::codegen::serde::FXCGenSerde;
use crate::codegen::{FXCodeGenContextual, FXCodeGenCtx, FXFieldCtx, FXHelperKind, FXInlining, FXValueRepr};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::spanned::Spanned;

pub trait FXSyncImplDetails {
    fn async_decl(&self) -> TokenStream;
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

pub struct FXCodeGenSync {
    ctx:        Rc<FXCodeGenCtx>,
    impl_async: impl_async::FXAsyncImplementor,
    impl_sync:  impl_sync::FXSyncImplementor,
}

impl FXCodeGenSync {
    pub fn new(ctx: Rc<FXCodeGenCtx>) -> Self {
        Self {
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
        let method_name = self.helper_name(fctx, helper_kind)?;
        let ident = fctx.ident_tok();
        let vis_tok = fctx.vis_tok(helper_kind);
        let ty = fctx.ty();
        let attributes_fn = self.attributes_fn(fctx, helper_kind, FXInlining::Always);
        let rwlock_guard = self.implementor(fctx).rwlock_read_guard();
        let async_decl = self.implementor(fctx).async_decl();
        let await_call = self.implementor(fctx).await_call();
        let read_method = self.read_method_name(fctx, false, &span);

        Ok(if fctx.is_lazy() {
            let read_arg = self.maybe_ref_counted_self(fctx);
            let implementor = self.implementor(fctx);
            let async_decl = implementor.async_decl();
            let await_call = implementor.await_call();
            let mapped_guard = self.implementor(fctx).rwlock_mapped_read_guard();
            let fallible_ty =
                self.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<'fx_reader_lifetime, #ty> })?;

            quote_spanned! [span=>
                #attributes_fn
                #vis_tok #async_decl fn #method_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> #fallible_ty {
                    self.#ident.#read_method(&#read_arg)#await_call
                }
            ]
        }
        else if fctx.is_optional() {
            let ty = quote_spanned! {span=> #rwlock_guard<'fx_reader_lifetime, ::std::option::Option<#ty>> };

            quote_spanned! [span=>
                #attributes_fn
                #vis_tok #async_decl fn #method_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> #ty {
                    self.#ident.#read_method()#await_call
                }
            ]
        }
        else {
            let ty = quote_spanned! {span=> #rwlock_guard<#ty> };

            quote_spanned! [span=>
                #attributes_fn
                #vis_tok #async_decl fn #method_name(&self) -> #ty {
                    self.#ident.read()#await_call
                }
            ]
        })
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

impl FXCodeGenContextual for FXCodeGenSync {
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
            let is_optional = fctx.is_optional();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Accessor, FXInlining::Always);
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let accessor_name = self.accessor_name(fctx)?;
            let ty = fctx.ty();
            let read_method = self.read_method_name(fctx, false, &span);

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
                    let async_decl = implementor.async_decl();
                    let await_call = implementor.await_call();
                    let read_arg = self.maybe_ref_counted_self(fctx);
                    let ty = self.fallible_return_type(fctx, ty)?;
                    let ret = fctx.fallible_ok_return(quote_spanned! {span=> (*rlock)#cmethod});

                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok #async_decl fn #accessor_name(&self) -> #ty {
                            let rlock = self.#ident.#read_method(&#read_arg) #await_call #shortcut;
                            #ret
                        }
                    ]
                }
                else if fctx.needs_lock() {
                    let ty = self.maybe_optional_ty(fctx, ty);

                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            let rlock = self.#ident.read() #shortcut;
                            (*rlock)#cmethod
                        }
                    ]
                }
                else if is_optional {
                    let ty = self.maybe_optional_ty(fctx, ty);
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            self.#ident #cmethod
                        }
                    ]
                }
                else {
                    let cmethod = if fctx.is_copy() { quote![] } else { quote![.clone()] };
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            self.#ident #cmethod
                        }
                    ]
                }
            }
            else if is_lazy || fctx.needs_lock() {
                self.field_reader_method(fctx, FXHelperKind::Accessor)?
            }
            else {
                let ty = self.fallible_return_type(fctx, &self.maybe_optional_ty(fctx, ty))?;

                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> &#ty {
                        &self.#ident
                    }
                ]
            }
        }
        else {
            quote![]
        })
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor_mut() {
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let accessor_name = self.accessor_mut_name(fctx)?;
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Accessor, FXInlining::Always);
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let read_method = self.read_method_name(fctx, true, &span);

            if fctx.is_lazy() {
                let read_arg = self.maybe_ref_counted_self(fctx);
                let implementor = self.implementor(fctx);
                let async_decl = implementor.async_decl();
                let await_call = implementor.await_call();
                let mapped_guard = self.implementor(fctx).rwlock_mapped_write_guard();
                let ty = self.fallible_return_type(fctx, quote_spanned! {span=> #mapped_guard<'fx_get_mut, #ty> })?;

                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok #async_decl fn #accessor_name<'fx_get_mut>(&'fx_get_mut self) -> #ty {
                        self.#ident.#read_method(&#read_arg)#await_call
                    }
                ]
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
                    let ty_toks = quote_spanned! [lock_span=> #wrguard<#ty_toks>];

                    quote_spanned! [span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty_toks {
                            self.#ident.write()
                        }
                    ]
                }
                else {
                    // Bare field
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&mut self) -> &mut #ty_toks {
                            &mut self.#ident
                        }
                    ]
                }
            }
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

    fn field_lazy_initializer(
        &self,
        fctx: &FXFieldCtx,
        self_ident: Option<TokenStream>,
    ) -> darling::Result<TokenStream> {
        let ident = fctx.ident_tok();
        let self_var = self_ident.unwrap_or(quote![self]);
        Ok(quote![#self_var.#ident.lazy_init(#self_var)])
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
            let writer_name = self.writer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Writer);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Writer, FXInlining::Always);
            let wrlock_guard = self.implementor(fctx).rwlock_write_guard();
            let async_decl = self.implementor(fctx).async_decl();
            let await_call = self.implementor(fctx).await_call();

            if fctx.is_lazy() {
                let fx_wrlock_guard = self.implementor(fctx).fx_mapped_write_guard();
                let builder_wrapper_type = self.builder_wrapper_type(fctx, false)?;
                quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok #async_decl fn #writer_name<'fx_writer_lifetime>(&'fx_writer_lifetime self) -> #fx_wrlock_guard<'fx_writer_lifetime, #builder_wrapper_type> {
                        self.#ident.write()#await_call
                    }
                ]
            }
            else if fctx.is_optional() {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok #async_decl fn #writer_name(&self) -> #wrlock_guard<::std::option::Option<#ty>> {
                        self.#ident.write()#await_call
                    }
                ]
            }
            else {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok #async_decl fn #writer_name(&self) -> #wrlock_guard<#ty> {
                        self.#ident.write()#await_call
                    }
                ]
            }
        }
        else {
            TokenStream::new()
        })
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_setter() {
            let span = self.helper_span(fctx, FXHelperKind::Setter);
            let set_name = self.setter_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Setter);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Setter, FXInlining::Always);
            let (gen_params, val_type, into_tok) = self.into_toks(fctx, fctx.is_setter_into());
            let needs_lock = fctx.needs_lock();
            let async_decl = self.implementor(fctx).async_decl();
            let await_call = self.implementor(fctx).await_call();

            if fctx.is_lazy() {
                quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok #async_decl fn #set_name #gen_params(&self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident.write()#await_call.store(value #into_tok)
                    }
                ]
            }
            else if fctx.is_optional() {
                let (lock_method, mutable, opt_async_decl, opt_await_call) = if needs_lock {
                    let lock_span = fctx.lock().span();
                    (quote_spanned![lock_span=> .write()], quote![], async_decl, await_call)
                }
                else {
                    (quote![], quote![mut], quote![], quote![])
                };
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok #opt_async_decl fn #set_name #gen_params(& #mutable self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident #lock_method #opt_await_call .replace(value #into_tok)
                    }
                ]
            }
            else if fctx.needs_lock() {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok #async_decl fn #set_name #gen_params(&self, value: #val_type) -> #ty {
                        let mut wlock = self.#ident.write()#await_call;
                        ::std::mem::replace(&mut *wlock, value #into_tok)
                    }
                ]
            }
            else {
                quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&mut self, value: #val_type) -> #ty {
                        ::std::mem::replace(&mut self.#ident, value #into_tok)
                    }
                ]
            }
        }
        else {
            TokenStream::new()
        })
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_clearer() {
            let span = self.helper_span(fctx, FXHelperKind::Clearer);
            let clear_name = self.clearer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Clearer);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Clearer, FXInlining::Always);
            let async_decl = self.implementor(fctx).async_decl();
            let await_call = self.implementor(fctx).await_call();

            if fctx.is_lazy() {
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok #async_decl fn #clear_name(&self) -> ::std::option::Option<#ty> {
                        self.#ident.clear()#await_call
                    }
                ]
            }
            else {
                // If not lazy then it's optional
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok #async_decl fn #clear_name(&self) -> ::std::option::Option<#ty> {
                       self.#ident.write()#await_call.take()
                    }
                ]
            }
        }
        else {
            TokenStream::new()
        })
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_predicate() {
            let span = self.helper_span(fctx, FXHelperKind::Predicate);
            let pred_name = self.predicate_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Predicate);
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Predicate, FXInlining::Always);

            if fctx.is_lazy() {
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #pred_name(&self) -> bool {
                        self.#ident.is_set()
                    }
                ]
            }
            // If not lazy then it's optional
            else if fctx.needs_lock() {
                let async_decl = self.implementor(fctx).async_decl();
                let await_call = self.implementor(fctx).await_call();
                let read_method = self.read_method_name(fctx, false, &span);
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok #async_decl fn #pred_name(&self) -> bool {
                      self.#ident.#read_method()#await_call.is_some()
                   }
                ]
            }
            else {
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #pred_name(&self) -> bool {
                      self.#ident.is_some()
                   }
                ]
            }
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
