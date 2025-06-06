use enum_dispatch::enum_dispatch;
use fieldx_aux::FXAccessorMode;
use fieldx_aux::FXProp;
use fieldx_aux::FXPropBool;
use fieldx_core::codegen::constructor::FXConstructor;
use fieldx_core::codegen::constructor::FXFieldConstructor;
use fieldx_core::codegen::constructor::FXFnConstructor;
use fieldx_core::types::helper::FXHelperKind;
use fieldx_core::types::meta::FXToksMeta;
use fieldx_core::types::meta::FXValueFlag;
use fieldx_core::types::FXInlining;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use std::rc::Rc;
use syn::spanned::Spanned;

use crate::util::std_default_expr_toks;

use super::derive_ctx::FXDeriveCodegenCtx;
use super::derive_ctx::FXDeriveFieldCtx;
use super::FXCodeGenPlain;
use super::FXCodeGenSync;
use super::FXValueRepr;

#[derive(Debug, Clone, Default)]
pub(crate) struct FXAccessorElements {
    // Reference symbol '&'
    pub(crate) reference:   TokenStream,
    // Type reference symbol '&'; say, if method returns a reference to the field.
    pub(crate) type_ref:    TokenStream,
    // Derefer symbol '*'
    pub(crate) dereference: TokenStream,
    // Method call such as '.clone()' or '.as_ref()'
    pub(crate) method:      TokenStream,
}

#[enum_dispatch]
pub(crate) enum FXCodeGenerator<'a> {
    ModePlain(FXCodeGenPlain<'a>),
    ModeSync(FXCodeGenSync<'a>),
}

#[enum_dispatch(FXCodeGenerator)]
pub(crate) trait FXCodeGenContextual {
    fn ctx(&self) -> &Rc<FXDeriveCodegenCtx>;

    // Actual code producers
    fn field_accessor(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_accessor_mut(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_builder_setter(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<TokenStream>;
    fn field_reader(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_writer(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_setter(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_clearer(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_predicate(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_lazy_builder_wrapper(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>>;
    fn field_value_wrap(&self, fctx: &FXDeriveFieldCtx, value: FXValueRepr<FXToksMeta>) -> darling::Result<FXToksMeta>;
    fn field_default_wrap(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta>;
    fn field_lazy_initializer(
        &self,
        fctx: &FXDeriveFieldCtx,
        method_constructor: &mut FXFnConstructor,
    ) -> darling::Result<TokenStream>;
    #[cfg(feature = "serde")]
    // How to move field from shadow struct
    fn field_from_shadow(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta>;
    #[cfg(feature = "serde")]
    // How to move field from the struct itself
    fn field_from_struct(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXToksMeta>;
    fn type_tokens<'s>(&'s self, fctx: &'s FXDeriveFieldCtx) -> darling::Result<&'s TokenStream>;

    fn maybe_ref_counted<TT: ToTokens>(&self, ty: &TT) -> TokenStream {
        let ctx = self.ctx();
        let ref_counted = ctx.arg_props().rc();
        if *ref_counted {
            let rc_span = ref_counted.final_span();
            let rc_type = ctx.impl_details().ref_count_strong(rc_span);
            return quote_spanned![rc_span=> #rc_type<#ty>];
        }

        ty.to_token_stream()
    }

    fn maybe_ref_counted_create<NT: ToTokens, IT: ToTokens>(
        &self,
        self_name: &NT,
        struct_init: &IT,
        post_build_ident: Option<syn::Ident>,
    ) -> TokenStream {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let post_construct = if let Some(init) = post_build_ident {
            let builder_has_error_type = arg_props.builder_has_error_type();
            let shortcut = if *builder_has_error_type {
                quote_spanned![init.span()=> ?]
            }
            else {
                quote![]
            };
            quote_spanned![init.span()=> .#init() #shortcut]
        }
        else {
            quote![]
        };

        let rc = arg_props.rc();

        if *rc {
            let rc_span = rc.final_span();
            let rc_type = ctx.impl_details().ref_count_strong(rc_span);
            let myself_field = arg_props.myself_field_ident();
            // Never move the post_construct call outside the new_cyclic closure!  The primary purpose of post_build is
            // to allow tweaking of the struct once it has been created.  Invoking it on a reference-counted container
            // makes this task difficult or impossible, depending on the constraints applied to the fields of interest.
            quote_spanned![rc_span=>
                #rc_type::new_cyclic(
                    |me| {
                        #self_name {
                            #myself_field: me.clone(),
                            #struct_init
                        }
                        #post_construct
                    }
                )
            ]
        }
        else {
            quote_spanned![self_name.span()=>
                #self_name {
                    #struct_init
                }
                #post_construct
            ]
        }
    }

    fn maybe_optional<T: ToTokens>(&self, fctx: &FXDeriveFieldCtx, ty: T) -> TokenStream {
        let opt = fctx.optional();
        if *opt {
            quote_spanned! {opt.final_span()=> ::std::option::Option<#ty>}
        }
        else {
            ty.to_token_stream()
        }
    }

    fn field_decl(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<()> {
        let mut constructor = fctx.take_constructor()?;

        constructor.set_vis(fctx.vis());

        if !*fctx.skipped() {
            constructor.set_type(self.type_tokens(fctx)?.clone());
        };

        self.ctx().add_field_decl(constructor);

        Ok(())
    }

    fn maybe_add_helper_method(
        &self,
        method: Option<FXFnConstructor>,
        helper_kind: FXHelperKind,
        fctx: &FXDeriveFieldCtx,
    ) -> darling::Result<()> {
        if let Some(mut method) = method {
            let ctx = self.ctx();
            let props = fctx.props().field_props();
            let literals = match helper_kind {
                FXHelperKind::Accessor => props.accessor_doc(),
                FXHelperKind::AccessorMut => props.accessor_mut_doc(),
                FXHelperKind::Reader => props.reader_doc(),
                FXHelperKind::Writer => props.writer_doc(),
                FXHelperKind::Setter => props.setter_doc(),
                FXHelperKind::Clearer => props.clearer_doc(),
                FXHelperKind::Predicate => props.predicate_doc(),
                _ => None,
            };

            if literals.is_some() {
                method.maybe_add_doc(literals)?;
            }
            else if (matches!(helper_kind, FXHelperKind::Accessor) && *fctx.accessor())
                || (matches!(helper_kind, FXHelperKind::AccessorMut) && *fctx.accessor_mut() && !*fctx.accessor())
            {
                // If there is no explicits doc subarg for accessor or accessor_mut then try using field docs if they are
                // present.  Give priority to the accessor and fallback to the mutable otherwise because it doesn't make
                // sense to duplicate the documentation and normally whatever is field's docs is what directly makes sense
                // for its accessor.
                method.add_attributes(props.doc().iter());
            }

            ctx.add_method(method);
        }

        Ok(())
    }

    fn field_methods(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<()> {
        if !*fctx.skipped() {
            let ctx = self.ctx();
            let impl_ctx = ctx.impl_ctx();

            self.maybe_add_helper_method(self.field_accessor(fctx)?, FXHelperKind::Accessor, fctx)?;
            self.maybe_add_helper_method(self.field_accessor_mut(fctx)?, FXHelperKind::AccessorMut, fctx)?;
            self.maybe_add_helper_method(self.field_reader(fctx)?, FXHelperKind::Reader, fctx)?;
            self.maybe_add_helper_method(self.field_writer(fctx)?, FXHelperKind::Writer, fctx)?;
            self.maybe_add_helper_method(self.field_setter(fctx)?, FXHelperKind::Setter, fctx)?;
            self.maybe_add_helper_method(self.field_clearer(fctx)?, FXHelperKind::Clearer, fctx)?;
            self.maybe_add_helper_method(self.field_predicate(fctx)?, FXHelperKind::Predicate, fctx)?;
            ctx.maybe_add_method(self.field_lazy_builder_wrapper(fctx)?);

            if *ctx.arg_props().builder_struct() {
                if let Some(mut bm) = self.field_builder(fctx)? {
                    // If the builder method for the field doesn't have its own doc, use the field's doc.
                    if let Some(literals) = fctx.props().field_props().builder_doc() {
                        bm.add_doc(literals)?;
                    }
                    else {
                        bm.add_attributes(fctx.props().field_props().doc().iter());
                    }
                    impl_ctx.add_builder_method(bm)?;
                }
                impl_ctx.add_builder_field(self.field_builder_field(fctx)?)?;
            }
        }

        Ok(())
    }

    fn field_default(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<()> {
        let skipped = fctx.skipped();
        let def_expr = if *skipped {
            let span = skipped.final_span();
            fctx.default_value()
                .map_or_else(|| std_default_expr_toks(span), |dv| dv.to_token_stream().into())
        }
        else {
            self.field_default_wrap(fctx)?
        };
        let ident = fctx.ident();
        let attributes = def_expr.attributes.clone();
        let def_toks = def_expr.to_token_stream();
        fctx.set_default_expr(def_expr.replace(quote_spanned! [ident.span()=>
            #( #attributes )*
            #ident: #def_toks
        ]));
        Ok(())
    }

    fn field_default_value(&self, fctx: &FXDeriveFieldCtx) -> FXValueRepr<FXToksMeta> {
        if let Some(default_value) = fctx.default_value() {
            FXValueRepr::Versatile(FXToksMeta::new(
                default_value.to_token_stream(),
                super::FXValueFlag::UserDefault,
            ))
        }
        else if *fctx.lazy() || *fctx.optional() {
            FXValueRepr::None
        }
        else {
            let span = self
                .ctx()
                .arg_props()
                .needs_default()
                .orig_span()
                .unwrap_or_else(|| fctx.span());
            FXValueRepr::Exact(std_default_expr_toks(span))
        }
    }

    fn accessor_elements(&self, fctx: &FXDeriveFieldCtx) -> FXAccessorElements {
        let accessor_mode = fctx.accessor_mode();
        let span = accessor_mode
            .orig_span()
            .unwrap_or_else(|| fctx.accessor().final_span());
        match **accessor_mode {
            FXAccessorMode::Copy => FXAccessorElements {
                dereference: quote_spanned![span=> *],
                ..Default::default()
            },
            FXAccessorMode::Clone => FXAccessorElements {
                method: quote_spanned![span=> .clone()],
                ..Default::default()
            },
            FXAccessorMode::AsRef => FXAccessorElements {
                type_ref: quote_spanned![span=> &],
                method: quote_spanned![span=> .as_ref()],
                ..Default::default()
            },
            FXAccessorMode::None => FXAccessorElements {
                reference: quote_spanned![span=> &],
                ..Default::default()
            },
        }
    }

    fn field_simple_lazy_initializer(
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

    fn field_builder(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<Option<FXFnConstructor>> {
        let builder = fctx.forced_builder().or(fctx.builder());
        Ok(if *builder {
            let span = builder.final_span();
            let mut builder_ident = fctx.builder_ident().clone();
            builder_ident.set_span(span);
            let mut mc = FXFnConstructor::new(builder_ident);
            let ident = fctx.ident();
            let (val_type, gen_params, into_tok) = self.to_toks(fctx, fctx.builder_into());

            mc.set_span(span)
                .set_vis(fctx.builder_method_visibility())
                .maybe_add_generic(gen_params)
                .set_self_mut(true)
                .add_param(quote_spanned! {span=> value: #val_type})
                .set_self_borrow(false)
                .set_ret_type(quote_spanned! {span=> Self})
                .add_statement(quote_spanned! {span=> self.#ident = ::std::option::Option::Some(value #into_tok);})
                .set_ret_stmt(quote_spanned! {span=> self});
            mc.add_attribute_toks(fctx.helper_attributes_fn(FXHelperKind::Builder, FXInlining::Always, span))?;

            let method_optional = fctx.builder_method_optional();

            if *method_optional {
                mc.add_attribute_toks(quote_spanned![method_optional.final_span()=> #[allow(unused)]])?;
            }

            Some(mc)
        }
        else {
            None
        })
    }

    fn field_builder_field(&self, fctx: &FXDeriveFieldCtx) -> darling::Result<FXFieldConstructor> {
        let span = fctx.span();
        let ty = fctx.ty();
        let ty = quote_spanned! {span=> ::std::option::Option<#ty>};
        let mut fc = FXFieldConstructor::new(fctx.ident().clone(), ty, span);
        fc.maybe_add_attributes(fctx.props().field_props().builder_attributes().map(|a| a.iter()));
        // A precautionary measure as this kind of fields are unlikely to be read. However, some attributes may affect
        // the validity of the builder, like when they refer to generic lifetimes.
        // A reminder to self: this used to be `!forced_builder && !builder`.
        let not_builder = fctx.forced_builder().or(fctx.builder()).not();
        if *not_builder {
            fc.add_attribute_toks(quote_spanned![not_builder.final_span()=> #[allow(dead_code)]])?;
        }
        Ok(fc)
    }

    // Ensure that we return an error or panic if the builder field is required but not set.
    fn field_builder_value_required(&self, fctx: &FXDeriveFieldCtx) {
        let builder_required = fctx.builder_required();
        let builder = fctx.builder();
        if !*fctx.builder_method_optional() {
            let field_ident = fctx.ident();
            let field_name = field_ident.to_string();
            let span = builder_required.or(builder).final_span();
            let ctx = self.ctx();
            let arg_props = ctx.arg_props();

            let field_set_error = if let Some(variant) = arg_props.builder_error_variant() {
                // If variant is explicitly specified then use it.
                quote_spanned! {span=> #variant(#field_name.into())}
            }
            else {
                let mut error_create = quote_spanned! {span=>
                    ::fieldx::error::FieldXError::uninitialized_field(#field_name.into())
                };

                if arg_props.builder_error_type().is_some() {
                    // If there is  no variant but custom error type is requested then we expect that custom type to
                    // implement From<FieldXError> trait.
                    error_create = quote_spanned! {span=>
                        ::std::convert::Into::into(#error_create)
                    };
                }

                error_create
            };

            fctx.set_builder_checker(quote_spanned![span=>
                if self.#field_ident.is_none() {
                    return ::std::result::Result::Err(#field_set_error)
                }
            ]);
        }
    }

    fn field_builder_value_for_set(
        &self,
        fctx: &FXDeriveFieldCtx,
        field_ident: &syn::Ident,
        span: &Span,
    ) -> FXToksMeta {
        let ctx = self.ctx();
        let span = *span;
        let optional = fctx.optional();
        let builder_required = fctx.builder_required();
        let alternative = if fctx.has_default_value() {
            self.field_default_wrap(fctx).map_or_else(
                |e| {
                    ctx.push_error(e);
                    None
                },
                Some,
            )
        }
        else if *optional && !*builder_required {
            self.field_value_wrap(
                fctx,
                FXValueRepr::Exact(quote_spanned![optional.final_span()=> ::std::option::Option::None].into()),
            )
            .map_or_else(
                |e| {
                    ctx.push_error(e);
                    None
                },
                Some,
            )
        }
        else {
            None
        };

        if let Some(alternative) = alternative {
            let mut field_manual_value = ctx.unique_ident_pfx("field_manual_value");
            field_manual_value.set_span(span);
            let manual_wrapped = ctx.ok_or_empty(self.field_value_wrap(
                fctx,
                FXValueRepr::Versatile(quote_spanned![span=> #field_manual_value].into()),
            ));
            let taker = quote_spanned! {span=> self.#field_ident.take()};

            if manual_wrapped.has_flag(FXValueFlag::ContainerWrapped) {
                if alternative.flags == FXValueFlag::StdDefault as u8 {
                    quote_spanned! {span=> #taker.map_or_default(|#field_manual_value| #manual_wrapped) }.into()
                }
                else {
                    FXToksMeta::from(quote_spanned! {span=>
                        #taker.map_or_else(
                            || #alternative,
                            |#field_manual_value| #manual_wrapped
                        )
                    })
                    // Clippy complains about `|| #alternative`, but this is acceptable here since we do not know what
                    // the expression will evaluate to. It might be costly to compute if it is not necessary.
                    .add_attribute(quote_spanned! {span=> #[allow(clippy::redundant_closure)]})
                }
            }
            else if alternative.has_flag(FXValueFlag::StdDefault) {
                quote_spanned! {span=> #taker.unwrap_or_default() }.into()
            }
            else {
                FXToksMeta::from(quote_spanned! {span=> #taker.unwrap_or_else(|| #alternative) })
                    .add_attribute(quote_spanned! {span=> #[allow(clippy::redundant_closure)]})
            }
        }
        else {
            // If no alternative init path provided then we just unwrap. It'd be either totally safe if builder checker
            // is set for this field, or won't be ever ran because of an earlier error in this method.
            ctx.ok_or_empty(self.field_value_wrap(
                fctx,
                FXValueRepr::Versatile(quote_spanned![span=> self.#field_ident.take().unwrap()].into()),
            ))
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

    fn fixup_self_type(&self, tokens: TokenStream) -> TokenStream {
        let ctx = self.ctx();
        let span = tokens.span();
        let mut fixed_tokens = TokenStream::new();
        let struct_ident = ctx.input_ident();
        let (_, generics, _) = ctx.input().generics().split_for_impl();

        for t in tokens.into_iter() {
            match t {
                TokenTree::Ident(ref ident) => {
                    if ident == "Self" {
                        fixed_tokens.extend(quote_spanned![span=> <#struct_ident #generics>]);
                    }
                    else {
                        fixed_tokens.extend(t.to_token_stream());
                    }
                }
                TokenTree::Group(ref group) => {
                    let mut group = proc_macro2::Group::new(group.delimiter(), self.fixup_self_type(group.stream()));
                    group.set_span(span);
                    fixed_tokens.extend(TokenTree::Group(group).to_token_stream())
                }
                _ => fixed_tokens.extend(t.to_token_stream()),
            }
        }

        quote_spanned![span=> #fixed_tokens]
    }

    // TokenStreams used to produce methods with Into support.
    fn to_toks(
        &self,
        fctx: &FXDeriveFieldCtx,
        use_into: FXProp<bool>,
    ) -> (TokenStream, Option<TokenStream>, Option<TokenStream>) {
        let ty = fctx.ty();
        if *use_into {
            let span = use_into.final_span();
            (
                quote_spanned![span=> FXVALINTO],
                Some(quote_spanned![span=> FXVALINTO: ::std::convert::Into<#ty>]),
                Some(quote_spanned![span=> .into()]),
            )
        }
        else {
            (ty.to_token_stream(), None, None)
        }
    }

    fn simple_field_build_setter(&self, fctx: &FXDeriveFieldCtx, field_ident: &syn::Ident, span: &Span) -> TokenStream {
        let set_toks = self.field_builder_value_for_set(fctx, field_ident, span);
        let attributes = &set_toks.attributes;

        quote_spanned![*span=>
            #( #attributes )*
            #field_ident: #set_toks
        ]
    }

    fn maybe_ref_counted_self(&self, fctx: &FXDeriveFieldCtx, mc: &mut FXFnConstructor) -> darling::Result<()> {
        if mc.self_rc_ident().is_none() {
            let ctx = self.ctx();
            let arg_props = ctx.arg_props();
            let rc = arg_props.rc();
            if *rc {
                let myself_method = arg_props.myself_name();
                let span = rc.final_span();
                let self_rc = format_ident!("__fx_self_rc", span = span);
                let self_ident = mc.self_ident();
                let expect_msg = format!("Can't acquire strong reference to myself for field '{}'", fctx.ident());
                mc.add_statement(quote_spanned! {span=>
                    let #self_rc = #self_ident.#myself_method().expect(#expect_msg);
                });
                mc.set_self_rc_ident(self_rc.to_token_stream())?;
            }
        }

        Ok(())
    }

    #[inline]
    fn builder_return_type(&self) -> TokenStream {
        let ctx = self.ctx();
        let builder_ident = ctx.input_ident();
        let generic_params = ctx.struct_generic_params();
        quote![#builder_ident #generic_params]
    }
}
