use super::{
    constructor::method::MethodConstructor, FXAccessorMode, FXCodeGenContextual, FXCodeGenCtx, FXFieldCtx,
    FXHelperKind, FXValueRepr,
};
use crate::codegen::FXInlining;
use fieldx_aux::FXPropBool;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::spanned::Spanned;

pub struct FXCodeGenPlain<'a> {
    #[allow(dead_code)]
    codegen: &'a crate::codegen::FXRewriter<'a>,
    ctx:     Rc<FXCodeGenCtx>,
}

impl<'a> FXCodeGenPlain<'a> {
    pub fn new(codegen: &'a crate::codegen::FXRewriter<'a>, ctx: Rc<FXCodeGenCtx>) -> Self {
        let ctx = Rc::clone(&ctx);
        Self { ctx, codegen }
    }

    fn inner_mut_ref_type(&self, mutable: bool, span: Span) -> TokenStream {
        if mutable {
            quote_spanned![span=> ::fieldx::RefMut]
        }
        else {
            quote_spanned![span=> ::fieldx::Ref]
        }
    }

    fn maybe_inner_mut_accessor(&self, fctx: &FXFieldCtx, mc: &mut MethodConstructor) -> TokenStream {
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
        fctx: &FXFieldCtx,
        mc: &MethodConstructor,
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

    fn maybe_inner_mut_wrap(&self, fctx: &FXFieldCtx, expr: TokenStream) -> TokenStream {
        if *fctx.inner_mut() {
            let span = fctx.inner_mut().orig_span().unwrap_or_else(|| expr.span());
            quote_spanned! {span=> ::fieldx::RefCell::new(#expr)}
        }
        else {
            expr
        }
    }

    fn inner_mut_return_type(
        &self,
        fctx: &FXFieldCtx,
        mc: &mut MethodConstructor,
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

    fn get_or_init_method(&self, fctx: &FXFieldCtx, span: &Span) -> TokenStream {
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
    fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.ctx
    }

    fn field_lazy_builder_wrapper(&self, _: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(quote![])
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> darling::Result<&'s TokenStream> {
        Ok(fctx.ty_wrapped(|| {
            // fxtrace!(fctx.ident_tok().to_string());
            let mut ty_tok = fctx.ty().to_token_stream();
            let lazy = fctx.lazy();
            let optional = fctx.optional();
            let inner_mut = fctx.inner_mut();
            if *lazy {
                let span = lazy.final_span();
                ty_tok = quote_spanned![span=> ::fieldx::OnceCell<#ty_tok>];
            }
            else if *optional {
                let span = optional.final_span();
                ty_tok = quote_spanned![span=> ::std::option::Option<#ty_tok>];
            }

            if *inner_mut {
                let span = inner_mut.final_span();
                ty_tok = quote_spanned![span=> ::fieldx::RefCell<#ty_tok>];
            }

            ty_tok
        }))
    }

    fn ref_count_types(&self, span: Span) -> (TokenStream, TokenStream) {
        (
            quote_spanned![span=> ::std::rc::Rc],
            quote_spanned![span=> ::std::rc::Weak],
        )
    }

    fn field_lazy_initializer(&self, fctx: &FXFieldCtx, mc: &mut MethodConstructor) -> darling::Result<TokenStream> {
        let lazy_name = fctx.lazy_ident();
        let span = fctx.lazy().final_span();
        let init_method = self.get_or_init_method(fctx, &span);
        self.maybe_ref_counted_self(fctx, mc);
        let builder_self = mc.self_maybe_rc();
        Ok(quote_spanned! {span=> .#init_method (|| #builder_self.#lazy_name() ) })
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if *fctx.accessor() {
            let ident = fctx.ident();
            let mut mc = MethodConstructor::new(fctx.accessor_ident());
            let span = fctx.accessor().final_span();
            let accessor_mode = fctx.accessor_mode();
            let mode_span = accessor_mode.final_span();
            let is_copy = accessor_mode.is_copy();
            let is_clone = accessor_mode.is_clone();
            let lazy = fctx.lazy();
            let inner_mut = fctx.inner_mut();

            self.maybe_ref_counted_self(fctx, &mut mc);
            mc.set_span(span);
            mc.set_vis(fctx.accessor_visibility().to_token_stream());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Accessor, FXInlining::Always, span));

            #[rustfmt::skip]
            let (opt_ref, ty_ref, deref, meth) = match accessor_mode.value() {
                FXAccessorMode::Copy =>  (
                    quote![], quote![],
                    quote_spanned![mode_span=> *], quote![]
                ),
                FXAccessorMode::Clone => (
                    quote![], quote![],
                    quote![], quote_spanned![mode_span=> .clone()]
                ),
                FXAccessorMode::AsRef => (
                    quote![], quote_spanned![mode_span=> &],
                    quote![], quote_spanned![mode_span=> .as_ref()]
                ),
                FXAccessorMode::None => (
                    quote_spanned![mode_span=> &], quote![],
                    quote![], quote![]
                ),
            };

            if *lazy {
                let lazy_init = self.field_lazy_initializer(fctx, &mut mc)?;
                let accessor = self.maybe_inner_mut_accessor(fctx, &mut mc);
                let ret = self.maybe_inner_mut_map(fctx, &mc, accessor, Some(lazy_init));
                let shortcut = fctx.fallible_shortcut();
                let ty = fctx.ty();

                let ret_type = if *inner_mut && !(is_copy || is_clone) {
                    self.inner_mut_return_type(
                        fctx,
                        &mut mc,
                        self.fallible_return_type(fctx, quote_spanned! {span=> #ty})?,
                    )
                }
                else {
                    self.fallible_return_type(fctx, quote_spanned! {span=> #opt_ref #ty})?
                };
                mc.set_ret_type(ret_type);
                mc.set_ret_stmt(fctx.fallible_ok_return(&quote_spanned! {span=> #deref #ret #shortcut #meth}));
            }
            else {
                let mut ty = fctx.ty().to_token_stream();

                ty = self.maybe_optional(fctx, &quote_spanned! {span=> #ty_ref #ty});

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
                        (#deref self.#ident.borrow()) #meth
                    });
                }
                else {
                    mc.set_ret_type(quote_spanned! {span=> #opt_ref #ty});
                    mc.set_ret_stmt(quote_spanned! {span=> #opt_ref self.#ident #meth });
                }
            }
            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let accessor_mut = fctx.accessor_mut();
        Ok(if *accessor_mut {
            let ident = fctx.ident();
            let span = accessor_mut.final_span();
            let mut mc = MethodConstructor::new(fctx.accessor_mut_ident());
            let ty = fctx.ty();
            let inner_mut = fctx.inner_mut();

            self.maybe_ref_counted_self(fctx, &mut mc);

            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::AccessorMut, FXInlining::Always, span));
            mc.set_vis(fctx.accessor_mut_visibility());
            mc.set_ret_mut(true);
            mc.set_self_mut(!*inner_mut);

            if *fctx.lazy() {
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
                        self.fallible_return_type(fctx, quote_spanned! {span=> #ty})?,
                    )
                }
                else {
                    self.fallible_return_type(fctx, quote_spanned! {span=> &mut #ty})?
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
                    mc.set_ret_type(quote_spanned! {inner_mut_span=> ::fieldx::RefMut<#lifetime, #ty>});
                    mc.set_ret_stmt(quote_spanned! {inner_mut_span=> self.#ident.borrow_mut() });
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_type(quote_spanned! {span=> &mut #ty});
                    mc.set_ret_stmt(quote_spanned! {span=> &mut self.#ident });
                }
            }

            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let builder = fctx.builder().or(fctx.forced_builder());
        let span = builder.final_span();
        let field_ident = fctx.ident();
        let field_default = self.field_default_wrap(fctx)?;

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
                    quote_spanned! {lazy.final_span()=> ::fieldx::OnceCell::from(self.#field_ident.take().unwrap()) }
                }
                else {
                    // as_optional
                    quote_spanned! {optional.final_span()=> self.#field_ident.take()}
                };

                if *inner_mut {
                    field_value = quote_spanned! {inner_mut.final_span()=> ::fieldx::RefCell::new(#field_value)};
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

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let setter = fctx.setter();
        Ok(if *setter {
            let span = setter.span();
            let mut mc = MethodConstructor::new(fctx.setter_ident());
            let ident = fctx.ident();
            let ty = fctx.ty();
            let (val_type, gen_params, into_tok) = self.into_toks(fctx, fctx.setter_into());
            let mut value_tok = quote_spanned! {span=> value #into_tok};
            let optional = fctx.optional();
            let inner_mut = fctx.inner_mut();
            let lazy = fctx.lazy();

            mc.set_span(span);
            mc.set_vis(fctx.setter_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Setter, FXInlining::Always, span));
            mc.maybe_add_generic(gen_params);
            mc.add_param(quote_spanned! {span=> value: #val_type });

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

            mc.to_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let clearer = fctx.clearer();
        if *clearer {
            let span = clearer.final_span();
            let clearer_name = fctx.clearer_ident();
            let ident = fctx.ident();
            let vis_tok = fctx.clearer_visibility();
            let attributes_fn = fctx.helper_attributes_fn(FXHelperKind::Clearer, FXInlining::Always, span);
            let ty_tok = fctx.ty();

            let (mut_tok, ty_tok) = if *fctx.inner_mut() {
                (quote![], self.maybe_optional(fctx, ty_tok))
            }
            else {
                (
                    quote_spanned![span=> mut],
                    quote_spanned![span=> ::std::option::Option<#ty_tok>],
                )
            };

            Ok(quote_spanned! [span=>
                #attributes_fn
                #vis_tok fn #clearer_name(& #mut_tok self) -> #ty_tok {
                    self.#ident.take()
                }
            ])
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let lazy = fctx.lazy();
        let optional = fctx.optional();
        let predicate = fctx.predicate();

        if *predicate && (*lazy || *optional) {
            let predicate_ident = fctx.predicate_ident();
            let span = predicate.final_span();
            let ident = fctx.ident();
            let vis_tok = fctx.predicate_visibility();
            let attributes_fn = fctx.helper_attributes_fn(FXHelperKind::Predicate, FXInlining::Always, span);

            if *lazy {
                return Ok(quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #predicate_ident(&self) -> bool {
                        self.#ident.get().is_some()
                    }
                ]);
            }
            else {
                let inner_mut = fctx.inner_mut();
                let borrow_tok = if *inner_mut {
                    quote_spanned![inner_mut.final_span()=> .borrow()]
                }
                else {
                    quote![]
                };
                if *optional {
                    return Ok(quote_spanned! [span=>
                        #attributes_fn
                        #vis_tok fn #predicate_ident(&self) -> bool {
                            self.#ident #borrow_tok .is_some()
                        }
                    ]);
                }
            }
        }

        Ok(TokenStream::new())
    }

    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: FXValueRepr<TokenStream>) -> darling::Result<TokenStream> {
        let ident_span = fctx.ident().span();
        let lazy = fctx.lazy();
        let optional = fctx.optional();
        let is_inner_mut = *fctx.inner_mut();

        Ok(match value {
            FXValueRepr::Exact(v) => quote_spanned![ident_span=> #v],
            FXValueRepr::Versatile(v) => self.maybe_inner_mut_wrap(
                fctx,
                if *lazy {
                    quote_spanned![lazy.final_span()=> ::fieldx::OnceCell::from(#v)]
                }
                else if *optional || is_inner_mut {
                    let mut value_tok = v;

                    if *optional {
                        value_tok = quote_spanned![optional.final_span()=> ::std::option::Option::Some(#value_tok)];
                    }

                    value_tok
                }
                else {
                    quote_spanned![ident_span=> #v]
                },
            ),
            FXValueRepr::None => {
                if *lazy {
                    self.maybe_inner_mut_wrap(fctx, quote_spanned![lazy.final_span()=> ::fieldx::OnceCell::new()])
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

                    self.maybe_inner_mut_wrap(fctx, value_tok)
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

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.field_value_wrap(fctx, self.field_default_value(fctx).map(|dv| self.fixup_self_type(dv)))
    }

    // Reader/writer make no sense for non-sync. Hence do nothing.
    fn field_reader(&self, _fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(quote![])
    }

    fn field_writer(&self, _fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(quote![])
    }

    #[cfg(feature = "serde")]
    fn field_from_shadow(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident();
        let shadow_var = self.ctx().shadow_var_ident();
        let span = fctx.serde().final_span();

        Ok(if *fctx.serde_optional() {
            let default_value = self.field_default_wrap(fctx)?;
            let shadow_value = self.field_value_wrap(fctx, FXValueRepr::Versatile(quote_spanned![span=> v]))?;
            quote_spanned![span=> #shadow_var.#field_ident.map_or_else(|| #default_value, |v| #shadow_value) ]
        }
        else if *fctx.inner_mut() {
            self.field_value_wrap(
                fctx,
                FXValueRepr::Versatile(quote_spanned![span=> #shadow_var.#field_ident]),
            )?
        }
        else {
            quote_spanned![span=> #shadow_var.#field_ident]
        })
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident();
        let me_var = self.ctx().me_var_ident();
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
        })
    }
}
