use fieldx_aux::FXPropBool;
use fieldx_core::codegen::constructor::FXConstructor;
use fieldx_core::codegen::constructor::FXFnConstructor;
use fieldx_core::types::helper::FXHelperKind;
use fieldx_core::types::meta::FXToksMeta;
use fieldx_core::types::meta::FXValueFlag;
use fieldx_core::types::meta::FXValueMeta;
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
use super::FXCodeGenContextual;
use super::FXValueRepr;

pub(crate) struct FXCodeGenPlain<'a> {
    #[allow(dead_code)]
    codegen: &'a crate::codegen::FXRewriter<'a>,
    ctx:     Rc<FXDeriveCodegenCtx>,
}

impl<'a> FXCodeGenPlain<'a> {
    pub(crate) fn new(codegen: &'a crate::codegen::FXRewriter<'a>, ctx: Rc<FXDeriveCodegenCtx>) -> Self {
        let ctx = Rc::clone(&ctx);
        Self { ctx, codegen }
    }

    fn inner_mut_ref_type(&self, mutable: bool, span: Span) -> TokenStream {
        if mutable {
            quote_spanned![span=> ::fieldx::plain::RefMut]
        }
        else {
            quote_spanned![span=> ::fieldx::plain::Ref]
        }
    }

    fn maybe_inner_mut_accessor(&self, fctx: &FXDeriveFieldCtx, mc: &mut FXFnConstructor) -> TokenStream {
        let span = mc.span();
        let ident = fctx.ident();
        let self_ident = mc.self_ident();
        if *fctx.inner_mut() {
            let accessor_name = format_ident!("{}_ref", ident, span = ident.span());
            let borrow_method = if *mc.ret_mut() {
                quote_spanned! {span=> borrow_mut}
            }
            else {
                quote_spanned! {span=> borrow}
            };
            mc.add_statement(quote_spanned! {span=> let #accessor_name = #self_ident.#ident.#borrow_method();});
            accessor_name.to_token_stream()
        }
        else {
            quote_spanned! {span=> #self_ident.#ident}
        }
    }

    fn maybe_inner_mut_map(
        &self,
        fctx: &FXDeriveFieldCtx,
        mc: &FXFnConstructor,
        accessor: TokenStream,
        inner_method: Option<TokenStream>,
    ) -> TokenStream {
        let span = mc.span();
        if *fctx.inner_mut() {
            let ref_type = self.inner_mut_ref_type(*mc.ret_mut(), span);
            quote_spanned! {span=> #ref_type::map(#accessor, |inner| inner #inner_method)}
        }
        else {
            quote_spanned! {span=> #accessor #inner_method}
        }
    }

    fn maybe_inner_mut_wrap(
        &self,
        fctx: &FXDeriveFieldCtx,
        expr: FXValueMeta<TokenStream>,
    ) -> FXValueMeta<TokenStream> {
        let inner_mut = fctx.inner_mut();
        if *inner_mut {
            let span = inner_mut.orig_span().unwrap_or_else(|| expr.span());
            expr.clone()
                .replace(quote_spanned! {span=> ::fieldx::plain::RefCell::new(#expr)})
                .mark_as(FXValueFlag::ContainerWrapped)
        }
        else {
            expr
        }
    }

    fn inner_mut_return_type(
        &self,
        fctx: &FXDeriveFieldCtx,
        mc: &mut FXFnConstructor,
        ret_type: TokenStream,
    ) -> TokenStream {
        if *fctx.inner_mut() {
            let span = mc.span();
            let lifetime = if let Some(lf) = mc.self_lifetime() {
                lf
            }
            else {
                mc.set_self_lifetime(quote_spanned![span=> 'fx_reader_lifetime]);
                mc.self_lifetime().as_ref().unwrap()
            };
            let ref_type = self.inner_mut_ref_type(*mc.ret_mut(), span);
            quote_spanned! {span=> #ref_type<#lifetime, #ret_type>}
        }
        else {
            ret_type
        }
    }

    fn get_or_init_method(&self, fctx: &FXDeriveFieldCtx, span: &Span) -> TokenStream {
        if *fctx.fallible() {
            quote_spanned! {*span=> get_or_try_init}
        }
        else {
            quote_spanned! {*span=> get_or_init}
        }
    }
}

impl<'a> FXCodeGenContextual for FXCodeGenPlain<'a> {
    #[inline(always)]
    fn ctx(&self) -> &Rc<FXDeriveCodegenCtx> {
        &self.ctx
    }

    fn field_lazy_builder_wrapper(&self, _: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        Ok(None)
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXDeriveFieldCtx) -> darling::Result<&'s TokenStream> {
        fctx.ty_wrapped(|| {
            // fxtrace!(fctx.ident_tok().to_string());
            let mut ty_tok = fctx.ty().to_token_stream();
            let lazy = fctx.lazy();
            let optional = fctx.optional();
            let inner_mut = fctx.inner_mut();
            let implementor = fctx.impl_details();
            if *lazy {
                let span = lazy.final_span();
                let proxy_type = implementor.field_simple_proxy_type(span);
                ty_tok = quote_spanned![span=> #proxy_type<#ty_tok>];
            }
            else if *optional {
                let span = optional.final_span();
                ty_tok = quote_spanned![span=> ::std::option::Option<#ty_tok>];
            }

            if *inner_mut {
                let span = inner_mut.final_span();
                ty_tok = quote_spanned![span=> ::fieldx::plain::RefCell<#ty_tok>];
            }

            Ok(ty_tok)
        })
    }

    fn field_lazy_initializer(
        &self,
        fctx: &FXDeriveFieldCtx,
        mc: &mut FXFnConstructor,
    ) -> darling::Result<TokenStream> {
        let lazy_name = fctx.lazy_ident();
        let span = mc.span();
        let init_method = self.get_or_init_method(fctx, &span);
        self.maybe_ref_counted_self(fctx, mc)?;
        let builder_self = mc.self_maybe_rc();
        Ok(quote_spanned! {span=> .#init_method (|| #builder_self.#lazy_name() ) })
    }

    fn field_accessor(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        Ok(if *fctx.accessor() {
            let ident = fctx.ident();
            let mut mc = FXFnConstructor::new(fctx.accessor_ident().clone());
            let span = fctx.accessor().final_span();
            let accessor_mode = fctx.accessor_mode();
            let is_copy = accessor_mode.is_copy();
            let is_clone = accessor_mode.is_clone();
            let lazy = fctx.lazy();
            let inner_mut = fctx.inner_mut();

            mc.set_span(span)
                .set_vis(fctx.accessor_visibility().to_token_stream())
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Accessor, FXInlining::Always, span))?;

            let FXAccessorElements {
                reference,
                dereference,
                method,
                type_ref,
            } = self.accessor_elements(fctx);

            if *lazy {
                self.maybe_ref_counted_self(fctx, &mut mc)?;
                let lazy_init = self.field_lazy_initializer(fctx, &mut mc)?;
                let accessor = self.maybe_inner_mut_accessor(fctx, &mut mc);
                let ret = self.maybe_inner_mut_map(fctx, &mc, accessor, Some(lazy_init));
                let shortcut = fctx.fallible_shortcut();
                let ty = fctx.ty();

                let ret_type = if *inner_mut && !(is_copy || is_clone) {
                    self.inner_mut_return_type(
                        fctx,
                        &mut mc,
                        fctx.fallible_return_type(fctx, quote_spanned! {span=> #ty})?,
                    )
                }
                else {
                    fctx.fallible_return_type(fctx, quote_spanned! {span=> #reference #ty})?
                };
                mc.set_ret_type(ret_type);
                mc.set_ret_stmt(fctx.fallible_ok_return(&quote_spanned! {span=> #dereference #ret #shortcut #method}));
            }
            else {
                let mut ty = fctx.ty().to_token_stream();

                ty = self.maybe_optional(fctx, &quote_spanned! {span=> #type_ref #ty});

                if *inner_mut {
                    let inner_mut_span = inner_mut.final_span();
                    let (deref, ty_tok) = if is_clone || is_copy {
                        // With copy/clone we don't return Ref
                        (quote![*], quote_spanned![inner_mut_span=> #ty])
                    }
                    else {
                        (quote![], self.inner_mut_return_type(fctx, &mut mc, ty))
                    };

                    mc.set_ret_type(ty_tok);
                    mc.set_ret_stmt(quote_spanned! {span =>
                        #[allow(unused_parens)]
                        (#deref self.#ident.borrow()) #method
                    });
                }
                else {
                    mc.set_ret_type(quote_spanned! {span=> #reference #ty});
                    mc.set_ret_stmt(quote_spanned! {span=> #reference self.#ident #method });
                }
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
            let ident = fctx.ident();
            let span = accessor_mut.final_span();
            let mut mc = FXFnConstructor::new(fctx.accessor_mut_ident().clone());
            let ty = fctx.ty();
            let inner_mut = fctx.inner_mut();

            mc.set_vis(fctx.accessor_mut_visibility())
                .set_ret_mut(true)
                .set_self_mut(!*inner_mut)
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::AccessorMut, FXInlining::Always, span))?;

            if *fctx.lazy() {
                self.maybe_ref_counted_self(fctx, &mut mc)?;
                let lazy_init = self.field_lazy_initializer(fctx, &mut mc)?;
                let accessor = self.maybe_inner_mut_accessor(fctx, &mut mc);
                let mut return_stmt = self.maybe_inner_mut_map(
                    fctx,
                    &mc,
                    accessor.clone(),
                    Some(quote_spanned! {span=> .get_mut().unwrap()}),
                );
                let mut shortcut = quote![];

                if *fctx.fallible() {
                    return_stmt = quote_spanned! {span=> Ok(#return_stmt) };
                    shortcut = quote_spanned![span=> ?];
                }

                mc.add_statement(quote_spanned! {span=>
                    let _ = #accessor #lazy_init #shortcut;
                });
                let ret_type = if *inner_mut {
                    self.inner_mut_return_type(
                        fctx,
                        &mut mc,
                        fctx.fallible_return_type(fctx, quote_spanned! {span=> #ty})?,
                    )
                }
                else {
                    fctx.fallible_return_type(fctx, quote_spanned! {span=> &mut #ty})?
                };
                mc.set_ret_type(ret_type);
                mc.set_ret_stmt(quote_spanned! {span=> #return_stmt });
            }
            else {
                let ty = self.maybe_optional(fctx, ty);

                if *inner_mut {
                    let inner_mut_span = inner_mut.final_span();
                    let lifetime = quote_spanned! {inner_mut_span=> 'fx_reader_lifetime};
                    mc.set_self_lifetime(lifetime.clone());
                    mc.set_ret_type(quote_spanned! {inner_mut_span=> ::fieldx::plain::RefMut<#lifetime, #ty>});
                    mc.set_ret_stmt(quote_spanned! {inner_mut_span=> self.#ident.borrow_mut() });
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_type(quote_spanned! {span=> &mut #ty});
                    mc.set_ret_stmt(quote_spanned! {span=> &mut self.#ident });
                }
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_builder_setter(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<TokenStream> {
        let builder = fctx.builder().or(fctx.forced_builder());
        let span = builder.final_span();
        let field_ident = fctx.ident();
        let field_default = self.field_default_wrap(fctx)?;
        let implementor = fctx.impl_details();

        Ok(if !*builder {
            quote_spanned![span=> #field_ident: #field_default]
        }
        else {
            let lazy = fctx.lazy();
            let optional = fctx.optional();
            let inner_mut = fctx.inner_mut();
            let as_optional = *optional && !*fctx.builder_required();
            if *lazy || as_optional {
                let mut field_value = if *lazy {
                    let wrapper_type = implementor.field_simple_proxy_type(lazy.final_span());
                    quote_spanned! {lazy.final_span()=> #wrapper_type::from(self.#field_ident.take().unwrap()) }
                }
                else {
                    // as_optional
                    quote_spanned! {optional.final_span()=> self.#field_ident.take()}
                };

                if *inner_mut {
                    field_value = quote_spanned! {inner_mut.final_span()=> ::fieldx::plain::RefCell::new(#field_value)};
                }

                quote_spanned! {span=>
                    #field_ident: if self.#field_ident.is_some() {
                        #field_value
                    }
                    else {
                        #field_default
                    }
                }
            }
            else {
                self.simple_field_build_setter(fctx, field_ident, &span)
            }
        })
    }

    fn field_setter(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let setter = fctx.setter();
        Ok(if *setter {
            let span = setter.span();
            let mut mc = FXFnConstructor::new(fctx.setter_ident().clone());
            let ident = fctx.ident();
            let ty = fctx.ty();
            let (val_type, gen_params, into_tok) = self.to_toks(fctx, fctx.setter_into());
            let mut value_tok = quote_spanned! {span=> value #into_tok};
            let optional = fctx.optional();
            let inner_mut = fctx.inner_mut();
            let lazy = fctx.lazy();

            mc.set_span(span)
                .set_vis(fctx.setter_visibility())
                .maybe_add_generic(gen_params)
                .add_param(quote_spanned! {span=> value: #val_type })
                .add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Setter, FXInlining::Always, span))?;

            if *lazy {
                let accessor = if *inner_mut {
                    let inner_mut_span = inner_mut.final_span();
                    let accessor_name = format_ident!("{}_ref", ident, span = inner_mut_span);
                    mc.add_statement(
                        quote_spanned! {inner_mut_span=> let mut #accessor_name = self.#ident.borrow_mut();},
                    );
                    accessor_name.to_token_stream()
                }
                else {
                    quote_spanned! {span=> self.#ident}
                };

                mc.set_self_mut(true);
                mc.set_ret_type(quote_spanned! {span=> ::std::option::Option<#ty>});
                mc.add_statement(quote_spanned! {span=>
                    let old = #accessor.take();
                    let _ = #accessor.set(#value_tok);
                });
                mc.set_ret_stmt(quote_spanned! {span=> old });
            }
            else {
                mc.set_ret_type(self.maybe_optional(fctx, ty));

                if *inner_mut || *optional {
                    if *inner_mut {
                        if *optional {
                            value_tok = quote_spanned![optional.final_span()=> Some(#value_tok) ];
                        }
                    }
                    else {
                        mc.set_self_mut(true);
                    }

                    let span = inner_mut.or(optional).final_span();
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident.replace(#value_tok) });
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_stmt(quote_spanned! {span=> ::std::mem::replace(&mut self.#ident, #value_tok) });
                }
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
            let attributes_fn = fctx.helper_attributes_fn(FXHelperKind::Clearer, FXInlining::Always, span);
            let mut ty_tok = fctx.ty().to_token_stream();

            mc.set_span(span)
                .set_vis(fctx.clearer_visibility())
                .add_attribute_toks(attributes_fn)?;

            if *fctx.inner_mut() {
                ty_tok = self.maybe_optional(fctx, ty_tok);
            }
            else {
                mc.set_self_mut(true);
                ty_tok = quote_spanned! {span=> ::std::option::Option<#ty_tok>};
            }

            mc.set_ret_type(ty_tok)
                .set_ret_stmt(quote_spanned! {span=> self.#ident.take() });

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_predicate(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let lazy = fctx.lazy();
        let optional = fctx.optional();
        let predicate = fctx.predicate();

        Ok(if *predicate {
            let mut mc = FXFnConstructor::new(fctx.predicate_ident().clone());
            let span = predicate.final_span();
            let ident = fctx.ident();
            let attributes_fn = fctx.helper_attributes_fn(FXHelperKind::Predicate, FXInlining::Always, span);

            mc.set_span(span)
                .set_vis(fctx.predicate_visibility())
                .set_ret_type(quote_spanned! {span=> bool})
                .add_attribute_toks(attributes_fn)?;

            if *lazy {
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident.get().is_some() });
            }
            else if *optional {
                let inner_mut = fctx.inner_mut();
                let borrow_tok = if *inner_mut {
                    quote_spanned![inner_mut.final_span()=> .borrow()]
                }
                else {
                    quote![]
                };
                mc.set_ret_stmt(quote_spanned! {span=> self.#ident #borrow_tok .is_some() });
            }
            else {
                return Err(
                    darling::Error::custom("Predicate requires the field to be lazy or optional").with_span(&span),
                );
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_value_wrap(&self, fctx: &FXDeriveFieldCtx, value: FXValueRepr<FXToksMeta>) -> darling::Result<FXToksMeta> {
        let ident_span = fctx.ident().span();
        let lazy = fctx.lazy();
        let optional = fctx.optional();
        let is_inner_mut = *fctx.inner_mut();
        let wrapper_type = fctx.impl_details().field_simple_proxy_type(lazy.final_span());

        Ok(match value {
            FXValueRepr::Exact(v) => v,
            FXValueRepr::Versatile(mut v) => {
                if *lazy {
                    let value_toks = v.to_token_stream();
                    v = v
                        .replace(quote_spanned![lazy.final_span()=> #wrapper_type::from(#value_toks)])
                        .mark_as(FXValueFlag::ContainerWrapped);
                }
                else if *optional {
                    let value_toks = v.to_token_stream();
                    v = v
                        .replace(quote_spanned! {optional.final_span()=> ::std::option::Option::Some(#value_toks)})
                        .mark_as(FXValueFlag::ContainerWrapped);
                }

                self.maybe_inner_mut_wrap(fctx, v)
            }
            FXValueRepr::None => {
                if *lazy {
                    self.maybe_inner_mut_wrap(fctx, quote_spanned![lazy.final_span()=> #wrapper_type::new()].into())
                }
                else if *optional || is_inner_mut {
                    let mut value_tok = quote![];
                    if *optional {
                        value_tok = quote_spanned![optional.final_span()=> ::std::option::Option::None];
                    }

                    if is_inner_mut && value_tok.is_empty() {
                        return Err(darling::Error::custom(format!(
                            "No value was supplied for internally mutable field {}",
                            fctx.ident()
                        ))
                        .with_span(&ident_span));
                    }

                    self.maybe_inner_mut_wrap(fctx, value_tok.into())
                }
                else {
                    return Err(darling::Error::custom(format!(
                        "No value was supplied for plain field {}",
                        fctx.ident()
                    ))
                    .with_span(&ident_span));
                }
            }
        })
    }

    fn field_default_wrap(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta> {
        self.field_value_wrap(
            fctx,
            self.field_default_value(fctx).map(|dv| {
                let dv_toks = dv.to_token_stream();
                dv.replace(self.fixup_self_type(dv_toks))
            }),
        )
    }

    // Reader/writer make no sense for non-sync. Hence do nothing.
    fn field_reader(&self, _fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        Ok(None)
    }

    fn field_writer(&self, _fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        Ok(None)
    }

    #[cfg(feature = "serde")]
    fn field_from_shadow(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta> {
        let field_ident = fctx.ident();
        let impl_ctx = self.ctx().impl_ctx();
        let shadow_var = impl_ctx.shadow_var_ident()?;
        let span = fctx.serde().final_span();

        Ok(if *fctx.serde_optional() {
            let default_value = self.field_default_wrap(fctx)?;
            let v_ident = self.ctx().unique_ident_pfx("v");
            let shadow_value =
                self.field_value_wrap(fctx, FXValueRepr::Versatile(quote_spanned![span=> #v_ident].into()))?;
            let shadow_toks = shadow_value.to_token_stream();
            if default_value.has_flag(FXValueFlag::StdDefault) {
                shadow_value
                    .replace(quote_spanned![span=> #shadow_var.#field_ident.map_or_default(|#v_ident| #shadow_toks) ])
            }
            else {
                shadow_value.replace(
                quote_spanned![span=> #shadow_var.#field_ident.map_or_else(|| #default_value, |#v_ident| #shadow_toks) ]
                ).add_attribute(quote_spanned! {span=> #[allow(clippy::redundant_closure)]})
            }
        }
        else if *fctx.inner_mut() {
            self.field_value_wrap(
                fctx,
                FXValueRepr::Versatile(quote_spanned![span=> #shadow_var.#field_ident].into()),
            )?
        }
        else {
            quote_spanned![span=> #shadow_var.#field_ident].into()
        })
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta> {
        let field_ident = fctx.ident();
        let impl_ctx = self.ctx().impl_ctx();
        let me_var = impl_ctx.me_var_ident()?;
        let lazy_or_inner_mut = fctx.lazy().or(fctx.inner_mut());
        let optional = fctx.optional();

        Ok(if *lazy_or_inner_mut {
            quote_spanned![lazy_or_inner_mut.final_span()=> #me_var.#field_ident.take()]
        }
        else if *optional {
            quote_spanned![optional.final_span()=> #me_var.#field_ident]
        }
        else {
            quote_spanned![fctx.serde().final_span()=> #me_var.#field_ident]
        }
        .into())
    }
}
