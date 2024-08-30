use super::{FXAccessorMode, FXCodeGenContextual, FXCodeGenCtx, FXFieldCtx, FXHelperKind, FXValueRepr};
#[cfg(feature = "serde")]
use crate::codegen::serde::FXCGenSerde;
use crate::codegen::FXInlining;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::rc::Rc;
use syn::spanned::Spanned;

pub struct FXCodeGenPlain {
    ctx: Rc<FXCodeGenCtx>,
}

impl FXCodeGenPlain {
    pub fn new(ctx: Rc<FXCodeGenCtx>) -> Self {
        let ctx = Rc::clone(&ctx);
        Self { ctx }
    }
}

impl FXCodeGenContextual for FXCodeGenPlain {
    #[inline(always)]
    fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.ctx
    }

    #[inline(always)]
    fn fxstruct_trait(&self) -> TokenStream {
        quote![::fieldx::traits::FXStructNonSync]
    }

    // fn initializers_combined(&self) -> TokenStream {
    //     TokenStream::new()
    // }

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> &'s TokenStream {
        fctx.ty_wrapped(|| {
            // fxtrace!(fctx.ident_tok().to_string());
            let mut ty_tok = fctx.ty_tok().clone();
            let span = ty_tok.span();
            if fctx.is_lazy() {
                return quote_spanned![span=> ::fieldx::OnceCell<#ty_tok>];
            }

            if fctx.is_optional() {
                let span = fctx.optional_span();
                ty_tok = quote_spanned![span=> ::std::option::Option<#ty_tok>];
            }

            if fctx.is_inner_mut() {
                let span = fctx.inner_mut_span();
                ty_tok = quote_spanned! [span=> ::fieldx::RefCell<#ty_tok>];
            }

            ty_tok
        })
    }

    fn ref_count_types(&self) -> (TokenStream, TokenStream) {
        (quote![::std::rc::Rc], quote![::std::rc::Weak])
    }

    fn field_lazy_initializer(
        &self,
        fctx: &FXFieldCtx,
        self_ident: Option<TokenStream>,
    ) -> darling::Result<TokenStream> {
        let lazy_name = self.lazy_name(fctx)?;
        let ident = fctx.ident_tok();
        let self_var = self_ident.as_ref().cloned().unwrap_or(quote![self]);
        let builder_self = self_ident.unwrap_or(self.maybe_ref_counted_self(fctx));
        Ok(quote![#self_var.#ident.get_or_init( || #builder_self.#lazy_name() )])
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor() {
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let ident = fctx.ident();
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let ty_tok = fctx.ty_tok().clone();
            let accessor_name = self.accessor_name(fctx)?;
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Accessor, FXInlining::Always);

            let (opt_ref, ty_ref, deref, meth) = match fctx.accessor_mode() {
                FXAccessorMode::Copy => (quote![], quote![], quote![*], quote![]),
                FXAccessorMode::Clone => (quote![], quote![], quote![], quote![.clone()]),
                FXAccessorMode::AsRef => (quote![], quote![&], quote![], quote![.as_ref()]),
                FXAccessorMode::None => (quote![&], quote![], quote![], quote![]),
            };

            if fctx.is_lazy() {
                let lazy_init = self.field_lazy_initializer(fctx, None)?;

                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> #opt_ref #ty_tok {
                        #deref #lazy_init #meth
                    }
                ]
            }
            else {
                let mut ty_tok = fctx.ty_tok().clone();

                ty_tok = self.maybe_optional(fctx, quote![#ty_ref #ty_tok]);

                if fctx.is_inner_mut() {
                    let (deref, ty_tok) = if fctx.is_clone() || fctx.is_copy() {
                        (quote![*], quote![#ty_tok])
                    }
                    else {
                        (quote![], quote![::fieldx::Ref<'fx_reader_lifetime, #ty_tok>])
                    };

                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> #ty_tok {
                            #[allow(unused_parens)]
                            (#deref self.#ident.borrow()) #meth
                        }
                    ]
                }
                else {
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #opt_ref #ty_tok { #opt_ref self.#ident #meth }
                    ]
                }
            }
        }
        else {
            TokenStream::new()
        })
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_accessor_mut() {
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::AccessorMut);
            let mut ty_tok = fctx.ty_tok().clone();
            let accessor_name = self.accessor_mut_name(fctx)?;
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::AccessorMut, FXInlining::Always);
            let span = self.helper_span(fctx, FXHelperKind::AccessorMut);

            if fctx.is_lazy() {
                let lazy_name = self.lazy_name(fctx)?;
                let builder_self = self.maybe_ref_counted_self(fctx);

                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&mut self) -> &mut #ty_tok {
                        self.#ident.get_or_init( || #builder_self.#lazy_name() );
                        self.#ident.get_mut().unwrap()
                    }
                ]
            }
            else {
                ty_tok = self.maybe_optional(fctx, ty_tok);

                if fctx.is_inner_mut() {
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::RefMut<'fx_reader_lifetime, #ty_tok> {
                            self.#ident.borrow_mut()
                        }
                    ]
                }
                else {
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&mut self) -> &mut #ty_tok { &mut self.#ident }
                    ]
                }
            }
        }
        else {
            TokenStream::new()
        })
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, FXHelperKind::Builder);
        let field_ident = fctx.ident_tok();
        let field_default = self.field_default_wrap(fctx)?;

        Ok(
            if !fctx.forced_builder() && (fctx.is_ignorable() || !fctx.needs_builder()) {
                quote![]
            }
            else if fctx.is_lazy() {
                quote_spanned![span=>
                    #field_ident: if self.#field_ident.is_some() {
                        ::fieldx::OnceCell::from(self.#field_ident.take().unwrap())
                    }
                    else {
                        #field_default
                    }
                ]
            }
            else if fctx.is_optional() && !fctx.is_builder_required() {
                // When there is a value from user then we use this code.
                let mut some_value = quote![self.#field_ident.take()];
                if fctx.is_inner_mut() {
                    some_value = quote![::fieldx::RefCell::from(#some_value)];
                }

                quote_spanned![span=>
                    #field_ident: if self.#field_ident.is_some() {
                            #some_value
                        }
                        else {
                            #field_default
                        }
                ]
            }
            else {
                self.simple_field_build_setter(fctx, field_ident, &span)
            },
        )
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.needs_setter() {
            let setter_name = self.setter_name(fctx)?;
            let span = self.helper_span(fctx, FXHelperKind::Setter);
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Setter);
            let mut ty_tok = fctx.ty_tok().clone();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Setter, FXInlining::Always);
            let (gen_params, val_type, into_tok) = self.into_toks(fctx, fctx.is_setter_into());

            if fctx.is_lazy() {
                quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #setter_name #gen_params(&mut self, value: #val_type) -> ::std::option::Option<#ty_tok> {
                        let old = self.#ident.take();
                        let _ = self.#ident.set(value #into_tok);
                        old
                    }
                ]
            }
            else {
                ty_tok = self.maybe_optional(fctx, ty_tok);

                if fctx.is_inner_mut() {
                    let value_tok = if fctx.is_optional() {
                        quote![Some(value #into_tok)]
                    }
                    else {
                        quote![value #into_tok]
                    };

                    quote_spanned! [span=>
                        #attributes_fn
                        #vis_tok fn #setter_name #gen_params(&self, value: #val_type) -> #ty_tok {
                            self.#ident.replace(#value_tok) }

                    ]
                }
                else if fctx.is_optional() {
                    quote_spanned! [span=>
                        #attributes_fn
                        #vis_tok fn #setter_name #gen_params(&mut self, value: #val_type) -> #ty_tok {
                            self.#ident.replace(value #into_tok)
                        }
                    ]
                }
                else {
                    quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #setter_name #gen_params(&mut self, value: #val_type) -> #ty_tok {
                            ::std::mem::replace(&mut self.#ident, value #into_tok)
                        }
                    ]
                }
            }
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
            FXValueRepr::Versatile(v) => {
                if is_lazy {
                    quote_spanned![ident_span=> ::fieldx::OnceCell::from(#v)]
                }
                else if is_optional || is_inner_mut {
                    let mut value_tok = v;

                    if is_optional {
                        value_tok = quote_spanned![ident_span=> ::std::option::Option::Some(#value_tok)];
                    }
                    if is_inner_mut {
                        value_tok = quote_spanned![ident_span=> ::fieldx::RefCell::from(#value_tok)];
                    }

                    value_tok
                }
                else {
                    quote_spanned![ident_span=> #v]
                }
            }
            FXValueRepr::None => {
                if is_lazy {
                    quote_spanned![ident_span=> ::fieldx::OnceCell::new()]
                }
                else if is_optional || is_inner_mut {
                    let mut value_tok = quote![];
                    if is_optional {
                        value_tok = quote_spanned![ident_span=> ::std::option::Option::None];
                    }

                    if is_inner_mut {
                        if value_tok.is_empty() {
                            return Err(darling::Error::custom(format!(
                                "No value was supplied for internally mutable field {}",
                                fctx.ident_str()
                            ))
                            .with_span(&ident_span));
                        }
                        else {
                            value_tok = quote_spanned![ident_span=> ::fieldx::RefCell::new(#value_tok)];
                        }
                    }
                    value_tok
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
        self.field_value_wrap(fctx, self.field_default_value(fctx)?.map(|dv| self.fixup_self_type(dv)))
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
