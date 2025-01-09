use super::{
    method_constructor::MethodConstructor, FXAccessorMode, FXCodeGenContextual, FXCodeGenCtx, FXFieldCtx, FXHelperKind,
    FXValueRepr,
};
#[cfg(feature = "serde")]
use crate::codegen::serde::FXCGenSerde;
use crate::codegen::FXInlining;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::{parse_quote_spanned, spanned::Spanned};

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
        let ident = fctx.ident_tok();
        let self_ident = mc.self_ident();
        if fctx.is_inner_mut() {
            let accessor_name = format_ident!("{}_ref", fctx.ident_str());
            let borrow_method = if *mc.ret_mut() {
                quote_spanned! {span=> borrow_mut}
            }
            else {
                quote_spanned! {span=> borrow}
            };
            mc.add_statement(quote_spanned! {span=> let mut #accessor_name = #self_ident.#ident.#borrow_method();});
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
        if fctx.is_inner_mut() {
            let ref_type = self.inner_mut_ref_type(*mc.ret_mut(), span);
            quote_spanned! {span=> #ref_type::map(#accessor, |inner| inner #inner_method)}
        }
        else {
            quote_spanned! {span=> #accessor #inner_method}
        }
    }

    fn maybe_inner_mut_wrap(&self, fctx: &FXFieldCtx, expr: TokenStream) -> TokenStream {
        if fctx.is_inner_mut() {
            let span = expr.span();
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
        if fctx.is_inner_mut() {
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
        if fctx.is_fallible() {
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
            let mut ty_tok = fctx.ty_tok().clone();
            if fctx.is_lazy() {
                let span = fctx.helper_span(FXHelperKind::Lazy);
                ty_tok = quote_spanned![span=> ::fieldx::OnceCell<#ty_tok>];
            }
            else if fctx.is_optional() {
                let span = fctx.optional_span();
                ty_tok = quote_spanned![span=> ::std::option::Option<#ty_tok>];
            }

            if fctx.is_inner_mut() {
                let span = fctx.inner_mut_span();
                ty_tok = quote_spanned! [span=> ::fieldx::RefCell<#ty_tok>];
            }

            ty_tok
        }))
    }

    fn ref_count_types(&self) -> (TokenStream, TokenStream) {
        (quote![::std::rc::Rc], quote![::std::rc::Weak])
    }

    fn field_lazy_initializer(&self, fctx: &FXFieldCtx, mc: &mut MethodConstructor) -> darling::Result<TokenStream> {
        let lazy_name = self.lazy_name(fctx)?;
        let span = self.helper_span(fctx, FXHelperKind::Lazy);
        let init_method = self.get_or_init_method(fctx, &span);
        self.maybe_ref_counted_self(fctx, mc);
        let builder_self = mc.self_maybe_rc();
        Ok(quote_spanned! {span=> .#init_method (|| #builder_self.#lazy_name() ) })
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor() {
            let ident = fctx.ident();
            let mut mc = MethodConstructor::new(self.accessor_name(fctx)?);
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let ty_tok = fctx.ty_tok().clone();
            let mode_span = fctx.accessor_mode_span().unwrap_or(span);
            let is_copy = fctx.is_copy();
            let is_clone = fctx.is_clone();

            self.maybe_ref_counted_self(fctx, &mut mc);
            mc.set_span(span);
            mc.set_vis(fctx.vis_tok(FXHelperKind::Accessor));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Accessor, FXInlining::Always));

            #[rustfmt::skip]
            let (opt_ref, ty_ref, deref, meth) = match fctx.accessor_mode() {
                FXAccessorMode::Copy =>  (quote![], quote![], quote_spanned![mode_span=> *], quote![]),
                FXAccessorMode::Clone => (quote![], quote![], quote![], quote_spanned![mode_span=> .clone()]),
                FXAccessorMode::AsRef => (quote![], quote_spanned![mode_span=> &], quote![], quote_spanned![mode_span=> .as_ref()]),
                FXAccessorMode::None =>  (quote_spanned![span=> &], quote![], quote![], quote![]),
            };

            if fctx.is_lazy() {
                let lazy_init = self.field_lazy_initializer(fctx, &mut mc)?;
                let accessor = self.maybe_inner_mut_accessor(fctx, &mut mc);
                let ret = self.maybe_inner_mut_map(fctx, &mc, accessor, Some(lazy_init));
                let shortcut = fctx.fallible_shortcut();

                let ret_type = if fctx.is_inner_mut() && !(is_copy || is_clone) {
                    self.inner_mut_return_type(
                        fctx,
                        &mut mc,
                        self.fallible_return_type(fctx, quote_spanned! {span=> #ty_tok})?,
                    )
                }
                else {
                    self.fallible_return_type(fctx, quote_spanned! {span=> #opt_ref #ty_tok})?
                };
                mc.set_ret_type(ret_type);
                mc.set_ret_stmt(fctx.fallible_ok_return(quote_spanned! {span=> #deref #ret #shortcut #meth}));
            }
            else {
                let mut ty_tok = fctx.ty_tok().clone();

                ty_tok = self.maybe_optional(fctx, quote_spanned! {span=> #ty_ref #ty_tok});

                if fctx.is_inner_mut() {
                    let inner_mut_span = fctx.inner_mut_span();
                    let (deref, ty_tok) = if is_clone || is_copy {
                        // With copy/clone we don't return Ref
                        (quote![*], quote_spanned![inner_mut_span=> #ty_tok])
                    }
                    else {
                        (quote![], self.inner_mut_return_type(fctx, &mut mc, ty_tok))
                    };

                    mc.set_ret_type(ty_tok);
                    mc.set_ret_stmt(quote_spanned! {span =>
                        #[allow(unused_parens)]
                        (#deref self.#ident.borrow()) #meth
                    });
                }
                else {
                    mc.set_ret_type(quote_spanned! {span=> #opt_ref #ty_tok});
                    mc.set_ret_stmt(quote_spanned! {span=> #opt_ref self.#ident #meth });
                }
            }
            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor_mut() {
            let ident = fctx.ident_tok();
            let span = self.helper_span(fctx, FXHelperKind::AccessorMut);
            let mut mc = MethodConstructor::new(self.accessor_mut_name(fctx)?);
            let mut ty_tok = fctx.ty_tok().clone();

            self.maybe_ref_counted_self(fctx, &mut mc);

            if let Some(attrs) = self.attributes_fn(fctx, FXHelperKind::AccessorMut, FXInlining::Always) {
                mc.add_attribute(attrs);
            }
            mc.set_vis(fctx.vis_tok(FXHelperKind::AccessorMut));
            mc.set_ret_mut(true);
            if !fctx.is_inner_mut() {
                mc.set_self_mut(true);
            }

            if fctx.is_lazy() {
                let lazy_init = self.field_lazy_initializer(fctx, &mut mc)?;
                let accessor = self.maybe_inner_mut_accessor(fctx, &mut mc);
                let mut return_stmt = self.maybe_inner_mut_map(
                    fctx,
                    &mc,
                    accessor.clone(),
                    Some(quote_spanned! {span=> .get_mut().unwrap()}),
                );
                let mut shortcut = quote![];

                if fctx.is_fallible() {
                    return_stmt = quote_spanned! {span=> Ok(#return_stmt) };
                    shortcut = quote_spanned![span=> ?];
                }

                mc.add_statement(quote_spanned! {span=>
                    let _ = #accessor #lazy_init #shortcut;
                });
                let ret_type = if fctx.is_inner_mut() {
                    self.inner_mut_return_type(
                        fctx,
                        &mut mc,
                        self.fallible_return_type(fctx, quote_spanned! {span=> #ty_tok})?,
                    )
                }
                else {
                    self.fallible_return_type(fctx, quote_spanned! {span=> &mut #ty_tok})?
                };
                mc.set_ret_type(ret_type);
                mc.set_ret_stmt(quote_spanned! {span=> #return_stmt });
            }
            else {
                ty_tok = self.maybe_optional(fctx, ty_tok);

                if fctx.is_inner_mut() {
                    let lifetime = quote_spanned! {span=> 'fx_reader_lifetime};
                    mc.set_self_lifetime(lifetime.clone());
                    mc.set_ret_type(parse_quote_spanned! {span=> ::fieldx::RefMut<#lifetime, #ty_tok>});
                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident.borrow_mut() });
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_type(quote_spanned! {span=> &mut #ty_tok});
                    mc.set_ret_stmt(quote_spanned! {span=> &mut self.#ident });
                }
            }

            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, FXHelperKind::Builder);
        let field_ident = fctx.ident_tok();
        let field_default = self.field_default_wrap(fctx)?;

        Ok(if !fctx.forced_builder() && !fctx.needs_builder() {
            quote_spanned![span=> #field_ident: #field_default]
        }
        else {
            let as_lazy = fctx.is_lazy();
            let as_optional = fctx.is_optional() && !fctx.is_builder_required();
            if as_lazy || as_optional {
                let mut field_value = if as_lazy {
                    quote_spanned! {span=> ::fieldx::OnceCell::from(self.#field_ident.take().unwrap()) }
                }
                else {
                    // as_optional
                    quote_spanned! {span=> self.#field_ident.take()}
                };

                if fctx.is_inner_mut() {
                    field_value = quote_spanned! {span=> ::fieldx::RefCell::new(#field_value)};
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
        Ok(if fctx.needs_setter() {
            let span = self.helper_span(fctx, FXHelperKind::Setter);
            let mut mc = MethodConstructor::new(self.setter_name(fctx)?);
            let ident = fctx.ident_tok();
            let ty_tok = fctx.ty_tok().clone();
            let (val_type, gen_params, into_tok) = self.into_toks(fctx, fctx.is_setter_into());
            let mut value_tok = quote_spanned! {span=> value #into_tok};
            let is_optional = fctx.is_optional();
            let is_inner_mut = fctx.is_inner_mut();

            mc.set_span(span);
            mc.set_vis(fctx.vis_tok(FXHelperKind::Setter));
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Setter, FXInlining::Always));
            mc.maybe_add_generic(gen_params);
            mc.add_param(quote_spanned! {span=> value: #val_type });

            if fctx.is_lazy() {
                let accessor = if is_inner_mut {
                    let inner_mut_span = fctx.inner_mut_span();
                    let accessor_name = format_ident!("{}_ref", fctx.ident_str(), span = inner_mut_span);
                    mc.add_statement(
                        quote_spanned! {inner_mut_span=> let mut #accessor_name = self.#ident.borrow_mut();},
                    );
                    accessor_name.to_token_stream()
                }
                else {
                    quote_spanned! {span=> self.#ident}
                };

                mc.set_self_mut(true);
                mc.set_ret_type(quote_spanned! {span=> ::std::option::Option<#ty_tok>});
                mc.add_statement(quote_spanned! {span=>
                    let old = #accessor.take();
                    let _ = #accessor.set(#value_tok);
                });
                mc.set_ret_stmt(quote_spanned! {span=> old });
            }
            else {
                mc.set_ret_type(self.maybe_optional(fctx, ty_tok));

                if is_inner_mut || is_optional {
                    if is_inner_mut {
                        if is_optional {
                            value_tok = quote_spanned![span=> Some(#value_tok) ];
                        }
                    }
                    else {
                        mc.set_self_mut(true);
                    }

                    mc.set_ret_stmt(quote_spanned! {span=> self.#ident.replace(#value_tok) });
                }
                else {
                    mc.set_self_mut(true);
                    mc.set_ret_stmt(quote_spanned! {span=> ::std::mem::replace(&mut self.#ident, #value_tok) });
                }
            }

            mc.into_method()
        }
        else {
            TokenStream::new()
        })
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_clearer() {
            let clearer_name = self.clearer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Clearer);
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Clearer, FXInlining::Always);
            let span = self.helper_span(fctx, FXHelperKind::Clearer);
            let ty_tok = fctx.ty_tok().clone();
            let (mut_tok, ty_tok) = if fctx.is_inner_mut() {
                (quote![], self.maybe_optional(fctx, ty_tok))
            }
            else {
                (quote![mut], quote![::std::option::Option<#ty_tok>])
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
        if fctx.needs_predicate() && (fctx.is_lazy() || fctx.is_optional()) {
            let predicate_name = self.predicate_name(fctx)?;
            let span = self.helper_span(fctx, FXHelperKind::Predicate);
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Predicate);
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Predicate, FXInlining::Always);

            if fctx.is_lazy() {
                return Ok(quote_spanned! [span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #predicate_name(&self) -> bool {
                        self.#ident.get().is_some()
                    }
                ]);
            }
            else {
                let borrow_tok = if fctx.is_inner_mut() {
                    quote![.borrow()]
                }
                else {
                    quote![]
                };
                if fctx.is_optional() {
                    return Ok(quote_spanned! [span=>
                        #attributes_fn
                        #vis_tok fn #predicate_name(&self) -> bool {
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
        let is_lazy = fctx.is_lazy();
        let is_optional = fctx.is_optional();
        let is_inner_mut = fctx.is_inner_mut();

        Ok(match value {
            FXValueRepr::Exact(v) => quote_spanned![ident_span=> #v],
            FXValueRepr::Versatile(v) => self.maybe_inner_mut_wrap(
                fctx,
                if is_lazy {
                    quote_spanned![ident_span=> ::fieldx::OnceCell::from(#v)]
                }
                else if is_optional || is_inner_mut {
                    let mut value_tok = v;

                    if is_optional {
                        value_tok = quote_spanned![ident_span=> ::std::option::Option::Some(#value_tok)];
                    }

                    value_tok
                }
                else {
                    quote_spanned![ident_span=> #v]
                },
            ),
            FXValueRepr::None => {
                if is_lazy {
                    self.maybe_inner_mut_wrap(fctx, quote_spanned![ident_span=> ::fieldx::OnceCell::new()])
                }
                else if is_optional || is_inner_mut {
                    let mut value_tok = quote![];
                    if is_optional {
                        value_tok = quote_spanned![ident_span=> ::std::option::Option::None];
                    }

                    if is_inner_mut && value_tok.is_empty() {
                        return Err(darling::Error::custom(format!(
                            "No value was supplied for internally mutable field {}",
                            fctx.ident_str()
                        ))
                        .with_span(&ident_span));
                    }

                    self.maybe_inner_mut_wrap(fctx, value_tok)
                }
                else {
                    return Err(darling::Error::custom(format!(
                        "No value was supplied for plain field {}",
                        fctx.ident_str()
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
        let field_ident = fctx.ident_tok();
        let shadow_var = self.ctx().shadow_var_ident();

        Ok(if self.is_serde_optional(fctx) {
            let default_value = self.field_default_wrap(fctx)?;
            let shadow_value = self.field_value_wrap(fctx, FXValueRepr::Versatile(quote![v]))?;
            quote![ #shadow_var.#field_ident.map_or_else(|| #default_value, |v| #shadow_value) ]
        }
        else if fctx.is_inner_mut() {
            let shadow_value = self.field_value_wrap(fctx, FXValueRepr::Versatile(quote![#shadow_var.#field_ident]))?;
            quote![ #shadow_value ]
        }
        else {
            quote![ #shadow_var.#field_ident ]
        })
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let me_var = self.ctx().me_var_ident();

        Ok(if fctx.is_lazy() || fctx.is_inner_mut() {
            quote![ #me_var.#field_ident.take() ]
        }
        else if fctx.is_optional() {
            quote![ #me_var.#field_ident ]
        }
        else {
            quote![ #me_var.#field_ident ]
        })
    }
}
