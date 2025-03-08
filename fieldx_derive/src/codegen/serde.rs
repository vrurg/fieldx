use super::{method_constructor::MethodConstructor, FXCodeGenContextual, FXFieldCtx};
use crate::helper::FXOrig;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;

pub trait FXCGenSerde: FXCodeGenContextual {
    fn filter_shadow_attributes<'a>(&'a self, fctx: &'a FXFieldCtx) -> impl Iterator<Item = &'a syn::Attribute> {
        // Only use `serde` attribute and those listed in forward_attrs
        let serde_helper = fctx
            .field()
            .serde()
            .as_ref()
            .or_else(|| self.ctx().args().serde().as_ref());

        fctx.attrs()
            .iter()
            .filter(move |a| a.path().is_ident("serde") || serde_helper.map_or(false, |sh| sh.accepts_attr(a)))
    }

    fn serde_skip_toks(&self, fctx: &FXFieldCtx) -> TokenStream {
        // Don't skip a field if:
        // - no `serde` argument
        // - it is not `serde(off)`
        // - and no more than one of `deserialize` or `serialize` is `off`
        if *self.ctx().arg_props().serde() {
            // let helper_span = fctx.serde().final_span();
            let serde = fctx.serde();
            if !*serde {
                return quote_spanned![serde.final_span()=> skip];
            }

            let serialize = fctx.serialize();
            if !*serialize {
                return quote_spanned![serialize.final_span()=> skip_serializing];
            }

            let deserialize = fctx.deserialize();
            if !*deserialize {
                return quote_spanned![deserialize.final_span()=> skip_deserializing];
            }
        }
        quote![]
    }

    fn serde_field_attribute(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        Ok(if *ctx.arg_props().serde() {
            let skip_toks = self.serde_skip_toks(fctx);
            let mut serde_attr_args = vec![];

            if !skip_toks.is_empty() {
                serde_attr_args.push(skip_toks);
            }

            let mut default_arg = None;

            if let Some(default_value) = fctx.serde_default_value() {
                // Safe because of has_default()
                let dv_span = default_value.final_span();

                if default_value.has_value() {
                    let serde_default_str: String = if default_value.is_str() {
                        default_value.try_into()?
                        // (&**default_value).try_into()?
                    }
                    else {
                        let struct_ident = ctx.input_ident();
                        let (_, generics, _) = ctx.input().generics().split_for_impl();
                        let default_fn_ident = self.serde_field_default_fn(fctx)?;

                        format!(
                            "{}{}::{}",
                            struct_ident,
                            generics.as_turbofish().to_token_stream(),
                            default_fn_ident
                        )
                    };

                    default_arg = Some(quote_spanned![dv_span=> default = #serde_default_str]);
                }
                else {
                    default_arg = Some(quote_spanned![dv_span=> default]);
                }
            }

            if let Some(default_arg) = default_arg {
                serde_attr_args.push(default_arg);
            }

            // Refer directly to the field's base name, because FieldCtx::base_name() always returns a valid identifier.
            if let Some(base_name) = fctx.props().field_props().base_name() {
                let span = base_name.span();
                let base_name = base_name.to_string();
                serde_attr_args.push(quote_spanned![span=> rename = #base_name]);
            }

            if serde_attr_args.is_empty() {
                quote![]
            }
            else {
                let span = fctx.serde().final_span();
                quote_spanned![span=> #[serde( #( #serde_attr_args ),* )] ]
            }
        }
        else {
            quote![]
        })
    }

    fn serde_shadow_field_type(&self, fctx: &FXFieldCtx) -> TokenStream {
        let ty = fctx.ty().to_token_stream();
        let serde_optional = fctx.serde_optional();
        if *serde_optional {
            quote_spanned![serde_optional.final_span()=> ::std::option::Option<#ty> ]
        }
        else {
            ty
        }
    }

    fn serde_shadow_field_value(&self, fctx: &FXFieldCtx, value: TokenStream) -> TokenStream {
        let serde_optional = fctx.serde_optional();
        if *serde_optional {
            quote_spanned![serde_optional.final_span()=> ::std::option::Option::Some(#value) ]
        }
        else {
            value
        }
    }

    fn serde_shadow_field(&self, fctx: &FXFieldCtx) {
        let ident = fctx.ident();
        let attrs = self.filter_shadow_attributes(fctx);
        let serde_attr = self.ok_or_empty(self.serde_field_attribute(fctx));
        let user_attrs = fctx.serde_attributes();
        let ty = self.serde_shadow_field_type(fctx);

        self.ctx()
            .add_shadow_field_decl(quote_spanned! [fctx.serde().final_span()=>
                #serde_attr
                #user_attrs
                #( #attrs )*
                #ident: #ty
            ]);
    }

    fn serde_shadow_field_default(&self, fctx: &FXFieldCtx) {
        let ctx = self.ctx();
        let needs_default = ctx.needs_default();

        if *needs_default {
            let field_ident = fctx.ident();
            let span = fctx.serde().final_span();

            let default_tok = self.fixup_self_type(
                self.field_default_value(fctx)
                    .map(|v| self.serde_shadow_field_value(fctx, v))
                    .unwrap_or_else(|| {
                        let serde_optional = fctx.serde_optional();
                        if *serde_optional {
                            quote_spanned![serde_optional.final_span()=> ::std::option::Option::None ]
                        }
                        else {
                            quote_spanned![span=> ::std::default::Default::default() ]
                        }
                    }),
            );

            self.ctx()
                .add_shadow_default_decl(quote_spanned![span=> #field_ident: #default_tok ]);
        }
    }

    fn serde_field_default_fn(&self, fctx: &FXFieldCtx) -> darling::Result<syn::Ident> {
        let field_type = self.serde_shadow_field_type(fctx);
        let serde = fctx.serde();
        if !*serde {
            return Err(darling::Error::custom(format!(
                "Can't generate default function for non-serde field {}",
                fctx.ident()
            )));
        }
        let Some(serde_default) = fctx.serde_default_value()
        else {
            return Err(darling::Error::custom(format!(
                "There is no serde 'default' for field {}",
                fctx.ident()
            )));
        };

        let span = serde.final_span();
        let mut fn_ident = fctx.default_fn_ident()?.clone();
        fn_ident.set_span(span);

        self.ctx().add_method_decl(quote_spanned![span=>
            #[allow(non_snake_case)]
            fn #fn_ident() -> #field_type {
                #serde_default
            }
        ]);
        Ok(fn_ident.clone())
    }
}

pub trait FXRewriteSerde<'a> {
    fn serde_derive_traits(&'a self) -> Vec<TokenStream>;
    fn serde_struct_attribute(&'a self) -> darling::Result<()>;
    fn serde_shadow_struct(&'a self) -> darling::Result<()>;
    fn serde_struct_from_shadow(&'a self);
    fn serde_struct_into_shadow(&'a self);
    fn serde_prepare_struct(&'a self);
    fn serde_rewrite_struct(&'a self);
    fn serde_shadow_default_fn(&'a self) -> darling::Result<Option<TokenStream>>;
}

impl<'a> FXRewriteSerde<'a> for super::FXRewriter<'a> {
    fn serde_derive_traits(&self) -> Vec<TokenStream> {
        let mut traits: Vec<TokenStream> = vec![];
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();

        let needs_serialize = arg_props.needs_serialize();
        if *needs_serialize {
            traits.push(quote_spanned![needs_serialize.final_span()=> Serialize]);
        }

        let needs_deserialize = arg_props.needs_deserialize();
        if *needs_deserialize {
            traits.push(quote_spanned![needs_deserialize.final_span()=> Deserialize]);
        }

        traits
    }

    fn serde_struct_attribute(&self) -> darling::Result<()> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();

        ctx.add_attr_from(if *serde {
            let mut serde_args: Vec<TokenStream> = vec![];

            let (_, generics, _) = ctx.input().generics().split_for_impl();
            let shadow_ident = format!(
                "{}{}",
                arg_props.serde_shadow_ident().unwrap(),
                generics.to_token_stream(),
            );
            let serde_span = serde.final_span();

            let needs_serialize = arg_props.needs_serialize();
            if *needs_serialize {
                let span = needs_serialize.final_span();
                serde_args.push(quote_spanned![span=> into = #shadow_ident]);
            }

            let needs_deserialize = arg_props.needs_deserialize();
            if *needs_deserialize {
                let span = needs_deserialize.final_span();
                serde_args.push(quote_spanned![span=> from = #shadow_ident]);
            }

            if serde_args.len() > 0 {
                quote_spanned![serde_span=> #[serde( #( #serde_args ),*  )] ]
            }
            else {
                quote![]
            }
        }
        else {
            quote![]
        });

        Ok(())
    }

    fn serde_shadow_struct(&'a self) -> darling::Result<()> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();
        if *serde {
            let span = serde.final_span();
            let shadow_ident = arg_props.serde_shadow_ident().unwrap();
            let fields = ctx.shadow_fields();
            let mut attrs = vec![];
            let derive_attr = crate::util::derive_toks(&self.serde_derive_traits());
            let (_, generics, where_clause) = ctx.input().generics().split_for_impl();
            let vis = arg_props.serde_visibility();
            let user_attributes = arg_props.serde_attributes();

            attrs.push(derive_attr);

            if let Some(default_attr_arg) = self.serde_shadow_default_fn()? {
                attrs.push(quote_spanned![default_attr_arg.span()=> #[serde(#default_attr_arg)]]);
            }

            let default_impl = if *arg_props.needs_default() {
                let shadow_defaults = ctx.shadow_defaults();
                quote_spanned! {span=>
                    impl #generics ::std::default::Default for #shadow_ident #generics #where_clause {
                        fn default() -> Self {
                            Self {
                                #( #shadow_defaults ),*
                            }
                        }
                    }
                }
            }
            else {
                quote![]
            };

            ctx.tokens_extend(quote_spanned![span=>
                #( #attrs )*
                #user_attributes
                #vis struct #shadow_ident #generics #where_clause {
                    #( #fields ),*
                }
                #default_impl
            ]);
        }

        Ok(())
    }

    // Impl From for the shadow struct
    fn serde_struct_from_shadow(&'a self) {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();
        if *serde && *arg_props.needs_deserialize() {
            let span = serde.final_span();
            let shadow_ident = arg_props.serde_shadow_ident().unwrap();
            let struct_ident = ctx.input_ident();
            let shadow_var = ctx.shadow_var_ident();
            let mut fields = vec![];
            let (_, generics, where_clause) = ctx.input().generics().split_for_impl();

            for fctx in ctx.all_field_ctx() {
                // if let Ok(fctx) = fctx {
                let deserialize = fctx.deserialize();
                if *fctx.serde() && *deserialize {
                    ctx.exec_or_record(|| {
                        let cgen = self.field_codegen(&fctx)?;
                        let field_ident = fctx.ident();
                        let fetch_shadow_field = cgen.field_from_shadow(&fctx)?;
                        fields.push(quote_spanned![deserialize.final_span()=>
                            #field_ident: #fetch_shadow_field
                        ]);
                        Ok(())
                    });
                }
            }

            let init_from_default = if *arg_props.needs_default() {
                quote_spanned![span=> .. Self::default()]
            }
            else {
                quote![]
            };

            ctx.tokens_extend(quote_spanned![span=>
                impl #generics ::std::convert::From<#shadow_ident #generics> for #struct_ident #generics #where_clause {
                    fn from(#shadow_var: #shadow_ident #generics) -> Self {
                        Self {
                            #( #fields, )*
                            #init_from_default
                        }
                    }
                }
            ]);
        }
    }

    fn serde_struct_into_shadow(&'a self) {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();
        if *serde && *arg_props.needs_serialize() {
            let span = serde.final_span();
            let mut mc = MethodConstructor::new(quote_spanned! {span=> from});
            let shadow_ident = arg_props.serde_shadow_ident().unwrap();
            let struct_ident = ctx.input_ident();
            let mut fields = vec![];
            let me_var = ctx.me_var_ident();
            let (_, generics, where_clause) = ctx.input().generics().split_for_impl();

            mc.set_self_ident(me_var.to_token_stream());
            mc.set_self_borrow(false);
            mc.set_self_type(Some(quote_spanned! {span=> #struct_ident #generics}));
            mc.set_ret_type(quote_spanned! {span=> Self});
            mc.set_span(span);
            mc.set_self_mut(true);

            for fctx in ctx.all_field_ctx() {
                let serialize = fctx.serialize();
                if *fctx.serde() && *serialize {
                    let field_ident = fctx.ident();

                    ctx.exec_or_record(|| {
                        let cgen = self.field_codegen(&fctx)?;
                        let fetch_struct_field = cgen.field_from_struct(&fctx)?;
                        let span = *fctx.span();

                        let lazy = fctx.lazy();
                        if *lazy {
                            let lazy_init = cgen.field_lazy_initializer(&fctx, &mut mc)?;
                            mc.add_statement(
                                quote_spanned![lazy.final_span()=> let _ = #me_var.#field_ident #lazy_init; ],
                            );
                        }

                        fields.push(quote_spanned![serialize.final_span()=> #field_ident: #fetch_struct_field ]);

                        Ok(())
                    });
                }
            }

            mc.set_ret_stmt(quote_spanned! {span=>
                Self {
                    #( #fields ),*
                }
            });
            let from_method = mc.into_method();

            ctx.tokens_extend(quote_spanned![span=>
                impl #generics ::std::convert::From<#struct_ident #generics> for #shadow_ident #generics #where_clause {
                    #from_method
                }
            ])
        }
    }

    fn serde_prepare_struct(&'a self) {
        let ctx = self.ctx();
        for fctx in ctx.all_field_ctx() {
            if *fctx.serde() {
                match self.field_codegen(&fctx) {
                    Ok(cgen) => {
                        cgen.serde_shadow_field(&fctx);
                        cgen.serde_shadow_field_default(&fctx);
                    }
                    Err(err) => ctx.push_error(err),
                }
            }
        }

        ctx.add_attr_from(crate::util::derive_toks(&self.serde_derive_traits()));

        ctx.ok_or_record(self.serde_struct_attribute());
    }

    fn serde_rewrite_struct(&'a self) {
        self.ctx().ok_or_record(self.serde_shadow_struct());
        self.serde_struct_from_shadow();
        self.serde_struct_into_shadow();
    }

    fn serde_shadow_default_fn(&self) -> darling::Result<Option<TokenStream>> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();

        if let Some(default_value) = arg_props.serde_default_value() {
            let span = default_value.orig().span();

            if default_value.has_value() {
                let default_span = default_value
                    .orig_span()
                    .unwrap_or_else(|| arg_props.serde().final_span());

                let serde_default: TokenStream = if default_value.is_str() {
                    // let default_str: String = (&**default_value).try_into()?;
                    let default_str: String = default_value.try_into()?;
                    let expr: syn::ExprPath = syn::parse_str(&default_str).map_err(|err| {
                        darling::Error::custom(format!("Invalid default string: {}", err)).with_span(&span)
                    })?;
                    quote_spanned![default_span=> #expr()]
                }
                else {
                    let default_code = default_value.value().cloned();
                    // if let NestedMeta::Meta(Meta::NameValue(_)) = default_code {
                    //     let err = darling::Error::custom(format!("Unexpected kind of argument")).with_span(&span);
                    //     #[cfg(feature = "diagnostics")]
                    //     let err = err.note(format!(
                    //         "{}\n{}\n{}",
                    //         "Consider using a string, as with serde `default`: \"Type::function\"`",
                    //         "                                       or a path: `Type::static_or_constant`",
                    //         "                       or a call-like expression: `Type::function()`"
                    //     ));
                    //     return Err(err);
                    // }
                    quote_spanned![default_span=> #default_code]
                };

                let generics = ctx.input().generics();
                let shadow_ident = arg_props.serde_shadow_ident().unwrap();
                let fn_ident = ctx.unique_ident_pfx(&format!("{}_default", shadow_ident));
                ctx.add_method_decl(quote_spanned![default_span=>
                    #[allow(non_snake_case)]
                    fn #fn_ident() -> #shadow_ident #generics {
                        #serde_default.into()
                    }
                ]);

                let default_str = format!("{}::{}", ctx.input_ident(), fn_ident);

                return Ok(Some(quote_spanned![span=> default = #default_str]));
            }
            else {
                return Ok(Some(quote_spanned![span=> default]));
            }
        }

        Ok(None)
    }
}

impl<T> FXCGenSerde for T where T: FXCodeGenContextual {}
