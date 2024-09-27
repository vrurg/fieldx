use super::{FXCodeGenCtx, FXFieldCtx, FXInlining};
#[cfg(feature = "serde")]
use crate::codegen::serde::FXCGenSerde;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::spanned::Spanned;

use super::{FXCodeGenContextual, FXHelperKind, FXValueRepr};

pub struct FXCodeGenSync {
    ctx: Rc<FXCodeGenCtx>,
}

impl FXCodeGenSync {
    pub fn new(ctx: Rc<FXCodeGenCtx>) -> Self {
        Self { ctx }
    }
}

impl FXCodeGenSync {
    #[inline]
    fn field_proxy_type(&self, _fctx: &FXFieldCtx) -> TokenStream {
        quote![FXProxy]
    }

    fn field_reader_method(&self, fctx: &FXFieldCtx, helper_kind: FXHelperKind) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, helper_kind);
        let method_name = self.helper_name(fctx, helper_kind)?;
        let ident = fctx.ident_tok();
        let vis_tok = fctx.vis_tok(helper_kind);
        let ty = fctx.ty();
        let attributes_fn = self.attributes_fn(fctx, helper_kind, FXInlining::Always);

        Ok(if fctx.is_lazy() {
            let read_arg = self.maybe_ref_counted_self(fctx);
            quote_spanned! [span=>
                #attributes_fn
                #vis_tok fn #method_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::MappedRwLockReadGuard<'fx_reader_lifetime, #ty> {
                    self.#ident.read(&#read_arg)
                }
            ]
        }
        else if fctx.is_optional() {
            quote_spanned! [span=>
                #attributes_fn
                #vis_tok fn #method_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::RwLockReadGuard<'fx_reader_lifetime, ::std::option::Option<#ty>> {
                    self.#ident.read()
                }
            ]
        }
        else {
            quote_spanned! [span=>
                #attributes_fn
                #vis_tok fn #method_name(&self) -> ::fieldx::RwLockReadGuard<#ty> {
                    self.#ident.read()
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
            quote_spanned![span=> ::fieldx::FXRwLock<#ty>]
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
}

impl FXCodeGenContextual for FXCodeGenSync {
    #[inline(always)]
    fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.ctx
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> &'s TokenStream {
        fctx.ty_wrapped(|| {
            let ty = fctx.ty_tok().clone();

            if fctx.is_skipped() {
                return ty;
            }

            let generic_params = self.generic_params();

            if fctx.is_lazy() {
                let ctx = self.ctx();
                let ident = ctx.input_ident();
                let proxy_type = self.field_proxy_type(fctx);
                let self_type = quote![#ident #generic_params]; // self.maybe_ref_counted(ctx, &quote![#ident #generic_params]);
                let span = fctx.ty().span();
                quote_spanned! [span=> ::fieldx::#proxy_type<#self_type, #ty>]
            }
            else {
                self.maybe_locked_ty(fctx, &self.maybe_optional_ty(fctx, &ty))
            }
        })
    }

    fn ref_count_types(&self) -> (TokenStream, TokenStream) {
        (quote![::std::sync::Arc], quote![::std::sync::Weak])
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

            if is_clone || is_copy {
                // unwrap won't panic because somewhere out there a copy/clone argument exists.
                let cc_span = fctx.accessor_mode_span().unwrap();
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
                    let read_arg = self.maybe_ref_counted_self(fctx);

                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            let rlock = self.#ident.read(&#read_arg);
                            (*rlock)#cmethod
                        }
                    ]
                }
                else if fctx.needs_lock() {
                    let ty = self.maybe_optional_ty(fctx, ty);
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            let rlock = self.#ident.read();
                            (*rlock) #cmethod
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
                let ty = self.maybe_optional_ty(fctx, ty);

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

            if fctx.is_lazy() {
                let read_arg = self.maybe_ref_counted_self(fctx);

                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name<'fx_get_mut>(&'fx_get_mut self) -> ::fieldx::MappedRwLockWriteGuard<'fx_get_mut, #ty> {
                        self.#ident.read_mut(&#read_arg)
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
                    let ty_toks = quote_spanned! [lock_span=> ::fieldx::RwLockWriteGuard<#ty_toks>];

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
        let input_type = self.input_type_toks();
        let field_type = self.type_tokens(fctx);
        let lazy_builder_name = self.lazy_name(fctx)?;
        let is_optional = fctx.is_optional();
        let is_lazy = fctx.is_lazy();
        let or_default = if fctx.has_default_value() {
            let default = self.fixup_self_type(self.field_default_value(fctx).expect(&format!(
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

        Ok(
            if !fctx.forced_builder() && (fctx.is_ignorable() || !fctx.needs_builder()) {
                quote![]
            }
            else if fctx.is_lazy() {
                quote_spanned![span=>
                    #field_ident: <#field_type>::new_default(
                        <#input_type>::#lazy_builder_name,
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
            },
        )
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

            if fctx.is_lazy() {
                quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #writer_name<'fx_writer_lifetime>(&'fx_writer_lifetime self) -> ::fieldx::FXWrLock<'fx_writer_lifetime, Self, #ty> {
                        self.#ident.write()
                    }
                ]
            }
            else if fctx.is_optional() {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #writer_name(&self) -> ::fieldx::RwLockWriteGuard<::std::option::Option<#ty>> {
                        self.#ident.write()
                    }
                ]
            }
            else {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #writer_name(&self) -> ::fieldx::RwLockWriteGuard<#ty> {
                        self.#ident.write()
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

            if fctx.is_lazy() {
                quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident.write().store(value #into_tok)
                    }
                ]
            }
            else if fctx.is_optional() {
                let (lock_method, mutable) = if needs_lock {
                    let lock_span = fctx.lock().span();
                    (quote_spanned![lock_span=> .write()], quote![])
                }
                else {
                    (quote![], quote![mut])
                };
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(& #mutable self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident #lock_method .replace(value #into_tok)
                    }
                ]
            }
            else if fctx.needs_lock() {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&self, value: #val_type) -> #ty {
                        let mut wlock = self.#ident.write();
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

            if fctx.is_lazy() {
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
                        self.#ident.clear()
                    }
                ]
            }
            else {
                // If not lazy then it's optional
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
                       self.#ident.write().take()
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
                quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #pred_name(&self) -> bool {
                      self.#ident.read().is_some()
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
            let input_type = self.input_type_toks();
            let field_type = self.type_tokens(fctx);
            let lazy_builder_name = self.lazy_name(fctx)?;

            quote_spanned![ident_span=> <#field_type>::new_default(<#input_type>::#lazy_builder_name, #value_wrapper) ]
        }
        else {
            if !is_optional && value.is_none() {
                return Err(darling::Error::custom(format!(
                    "No value was supplied for non-optional, non-lazy field '{}'",
                    fctx.ident_str()
                )));
            }

            if fctx.needs_lock() {
                quote_spanned![ident_span=> ::fieldx::FXRwLock::new(#value_wrapper) ]
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
