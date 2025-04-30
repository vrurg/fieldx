use crate::util::std_default_expr_toks;

use super::constructor::field::FXFieldConstructor;
use super::constructor::r#fn::FXFnConstructor;
use super::constructor::FXConstructor;
use super::constructor::FXImplConstructor;
use super::constructor::FXStructConstructor;
use super::FXCodeGenContextual;
use super::FXFieldCtx;
use super::FXToksMeta;
use super::FXValueFlag;
use fieldx_aux::FXOrig;
use fieldx_aux::FXProp;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use syn::spanned::Spanned;

fn serde_rename_attr(
    serialize_name: Option<&FXProp<String>>,
    deserialize_name: Option<&FXProp<String>>,
    span: Span,
) -> Option<TokenStream> {
    let mut args = vec![];
    for (arg, value) in [("serialize", serialize_name), ("deserialize", deserialize_name)] {
        if let Some(value) = value {
            let span = value.span();
            let value = value.to_string();
            let arg = format_ident!("{}", arg, span = span);
            args.push(quote_spanned![span=> #arg = #value]);
        }
    }
    if !args.is_empty() {
        Some(quote_spanned![span=> rename( #( #args ),* )])
    }
    else {
        None
    }
}

pub(crate) trait FXCGenSerde: FXCodeGenContextual {
    fn filter_shadow_attributes<'a>(&'a self, fctx: &'a FXFieldCtx) -> impl Iterator<Item = &'a syn::Attribute> {
        // Only use `serde` attribute and those listed in forward_attrs
        let serde_helper = fctx
            .field()
            .serde()
            .as_ref()
            .or_else(|| self.ctx().args().serde().as_ref());

        fctx.field()
            .attrs()
            .iter()
            .filter(move |a| a.path().is_ident("serde") || serde_helper.is_some_and(|sh| sh.accepts_attr(a)))
    }

    fn serde_skip_toks(&self, fctx: &FXFieldCtx) -> Option<TokenStream> {
        // Don't skip a field if:
        // - no `serde` argument
        // - it is not `serde(off)`
        // - and no more than one of `deserialize` or `serialize` is `off`
        let serde = fctx.serde();
        if !*serde {
            return Some(quote_spanned![serde.final_span()=> skip]);
        }

        let serialize = fctx.serialize();
        if !*serialize {
            return Some(quote_spanned![serialize.final_span()=> skip_serializing]);
        }

        let deserialize = fctx.deserialize();
        if !*deserialize {
            return Some(quote_spanned![deserialize.final_span()=> skip_deserializing]);
        }

        None
    }

    fn serde_field_attribute(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        Ok(if *ctx.arg_props().serde() {
            let span = fctx.serde().final_span();
            let mut serde_attr_args = vec![];

            if let Some(skip_toks) = self.serde_skip_toks(fctx) {
                serde_attr_args.push(skip_toks);
            }

            let mut default_arg = None;

            if let Some(default_value) = fctx.serde_default_value() {
                // Safe because of has_default()
                let dv_span = default_value.final_span();

                if default_value.has_value() {
                    let serde_default: TokenStream = if default_value.is_str() {
                        default_value.to_token_stream()
                        // (&**default_value).try_into()?
                    }
                    else {
                        let struct_ident = ctx.input_ident();
                        let (_, generics, _) = ctx.input().generics().split_for_impl();
                        let default_fn_ident = self.serde_field_default_fn(fctx)?;

                        syn::LitStr::new(
                            &format!(
                                "{}{}::{}",
                                struct_ident,
                                generics.as_turbofish().to_token_stream(),
                                default_fn_ident
                            ),
                            dv_span,
                        )
                        .to_token_stream()
                    };

                    default_arg = Some(quote_spanned![dv_span=> default = #serde_default]);
                }
                else {
                    default_arg = Some(quote_spanned![dv_span=> default]);
                }
            }

            if let Some(default_arg) = default_arg {
                serde_attr_args.push(default_arg);
            }

            if let Some(rename_arg) =
                serde_rename_attr(fctx.serde_rename_serialize(), fctx.serde_rename_deserialize(), span)
            {
                serde_attr_args.push(rename_arg);
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

    fn serde_shadow_field_value<FT: Into<FXToksMeta>>(&self, fctx: &FXFieldCtx, value: FT) -> FXToksMeta {
        let serde_optional = fctx.serde_optional();
        let mut value = value.into();
        if *serde_optional {
            let value_toks = value.to_token_stream();
            value = value
                .replace(quote_spanned![serde_optional.final_span()=> ::std::option::Option::Some(#value_toks) ])
                .mark_as(FXValueFlag::ContainerWrapped);
        }
        value
    }

    fn serde_shadow_field(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let mut fc = FXFieldConstructor::new(
            fctx.ident().clone(),
            self.serde_shadow_field_type(fctx),
            fctx.serde().final_span(),
        );

        fc.set_type(self.serde_shadow_field_type(fctx))
            .add_attributes(self.filter_shadow_attributes(fctx))
            .add_attribute_toks(self.ctx().ok_or_empty(self.serde_field_attribute(fctx)))?;
        if let Some(serde_attrs) = fctx.serde_attributes() {
            fc.add_attributes(serde_attrs.iter());
        }

        self.ctx().shadow_struct_mut()?.add_field(fc);

        Ok(())
    }

    fn serde_shadow_field_default(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let ctx = self.ctx();
        let needs_default = ctx.needs_default();

        if *needs_default {
            let field_ident = fctx.ident();
            let span = fctx.serde().final_span();
            let default_expr = self
                .field_default_value(fctx)
                .map(|v| self.serde_shadow_field_value(fctx, v))
                .unwrap_or_else(|| std_default_expr_toks(span));

            let default_toks = self.fixup_self_type(default_expr.to_token_stream());
            fctx.set_shadow_default_expr(default_expr.replace(quote_spanned! {span=> #field_ident: #default_toks}));
        }

        Ok(())
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

        let mut fn_ident = fctx.default_fn_ident()?.clone();
        let span = serde.final_span();
        let serde_default = self.serde_shadow_field_value(fctx, serde_default.to_token_stream());
        let mut mc = FXFnConstructor::new_associated(fn_ident.clone());

        fn_ident.set_span(span);

        mc.set_span(span)
            .set_ret_type(field_type)
            .add_attribute_toks(quote_spanned![span=> #[allow(non_snake_case)] ])?
            .set_ret_stmt(serde_default);

        self.ctx().add_method(mc);
        Ok(fn_ident)
    }
}

pub(crate) trait FXRewriteSerde<'a> {
    fn serde_derive_traits(&'a self) -> Vec<TokenStream>;
    fn serde_struct_attribute(&'a self) -> darling::Result<()>;
    fn serde_shadow_struct(&'a self) -> darling::Result<Option<FXStructConstructor>>;
    fn serde_struct_from_shadow(&'a self) -> darling::Result<()>;
    fn serde_struct_into_shadow(&'a self) -> darling::Result<()>;
    fn serde_prepare_struct(&'a self) -> darling::Result<()>;
    fn serde_rewrite_struct(&'a self);
    fn serde_shadow_field_default_fn(&'a self) -> darling::Result<Option<TokenStream>>;
    fn serde_shadow_default_impl(&'a self) -> darling::Result<()>;
    fn serde_finalize(&'a self) -> darling::Result<TokenStream>;
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

        ctx.user_struct_mut().add_attribute_toks(if *serde {
            let mut serde_args: Vec<TokenStream> = vec![];

            let (_, generics, _) = ctx.input().generics().split_for_impl();
            let mut shadow_ident_str = syn::LitStr::new(
                &format!(
                    "{}{}",
                    arg_props.serde_shadow_ident().unwrap(),
                    generics.to_token_stream(),
                ),
                serde.final_span(),
            );
            let serde_span = serde.final_span();

            let needs_serialize = arg_props.needs_serialize();
            if *needs_serialize {
                let span = needs_serialize.final_span();
                shadow_ident_str.set_span(span);
                serde_args.push(quote_spanned![span=> into = #shadow_ident_str]);
            }

            let needs_deserialize = arg_props.needs_deserialize();
            if *needs_deserialize {
                let span = needs_deserialize.final_span();
                shadow_ident_str.set_span(span);
                serde_args.push(quote_spanned![span=> from = #shadow_ident_str]);
            }

            if !serde_args.is_empty() {
                quote_spanned![serde_span=> #[serde( #( #serde_args ),*  )] ]
            }
            else {
                quote![]
            }
        }
        else {
            quote![]
        })?;

        Ok(())
    }

    fn serde_shadow_default_impl(&'a self) -> darling::Result<()> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();

        if *arg_props.needs_default() {
            let span = arg_props.serde().final_span();
            let mut shadow_defaults = Vec::new();
            let mut all_std = true;

            for fctx in ctx.all_field_ctx() {
                if *fctx.serde() {
                    let default = fctx
                        .shadow_default_expr()
                        .clone()
                        .unwrap_or(std_default_expr_toks(span));
                    // This is a standard default expression `Default::default()` when StdDefault is the only flag set.
                    all_std &= default.flags == FXValueFlag::StdDefault as u8;
                    shadow_defaults.push(default);
                }
            }

            // If all fields are initialized using unwrapped Default::default() then we can just derive the trait.
            if all_std {
                ctx.shadow_struct_mut()?
                    .add_attribute_toks(quote_spanned! {span=> #[derive(Default)]})?;
                return Ok(());
            }

            let ident_path: syn::Path = syn::parse2(quote_spanned! {span=> ::std::default::Default})?;
            let mut default_impl = FXImplConstructor::new(ident_path);

            default_impl
                .set_span(span)
                .set_from_generics(Some(ctx.input().generics().clone()))
                .set_for_ident(arg_props.serde_shadow_ident().cloned().unwrap());

            let mut default_fn = FXFnConstructor::new_associated(format_ident!("default", span = span));

            default_fn
                .set_span(span)
                .set_ret_type(format_ident!("Self", span = span))
                .set_ret_stmt(quote_spanned! {span=> Self { #( #shadow_defaults ),* } });

            default_impl.add_method(default_fn);
            ctx.shadow_struct_mut()?.add_trait_impl(default_impl);
        }

        Ok(())
    }

    fn serde_shadow_struct(&'a self) -> darling::Result<Option<FXStructConstructor>> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();

        Ok(if *serde {
            let mut shadow_struct = FXStructConstructor::new(arg_props.serde_shadow_ident().cloned().unwrap());
            let span = serde.final_span();
            let mut serde_attr_args = vec![];

            shadow_struct
                .set_span(span)
                .set_vis(arg_props.serde_visibility())
                .set_generics(ctx.input().generics().clone())
                .maybe_add_doc(arg_props.serde_doc())?
                .add_attribute_toks(crate::util::derive_toks(&self.serde_derive_traits()))?;

            if let Some(attrs) = arg_props.serde_attributes() {
                shadow_struct.add_attributes(attrs.iter());
            }

            if let Some(rename_attr) = serde_rename_attr(
                arg_props.serde_rename_serialize(),
                arg_props.serde_rename_deserialize(),
                span,
            ) {
                serde_attr_args.push(rename_attr);
            }

            if let Some(default_attr_arg) = self.serde_shadow_field_default_fn()? {
                serde_attr_args.push(default_attr_arg);
            }

            if !serde_attr_args.is_empty() {
                shadow_struct.add_attribute_toks(quote_spanned![span=> #[serde(#( #serde_attr_args ),*)]])?;
            }

            // ctx.tokens_extend(quote_spanned! {span=>
            //     #( #attrs )*
            //     #user_attributes
            //     #vis struct #shadow_ident #generics #where_clause {
            //         #( #fields ),*
            //     }
            //     #default_impl
            // });
            Some(shadow_struct)
        }
        else {
            None
        })
    }

    // Impl From for the shadow struct
    fn serde_struct_from_shadow(&'a self) -> darling::Result<()> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();
        if *serde && *arg_props.needs_deserialize() {
            let span = serde.final_span();
            let mut from_impl = FXImplConstructor::new(format_ident!("From", span = span));
            let mut from_method = FXFnConstructor::new_associated(format_ident!("from", span = span));
            let shadow_ident = arg_props.serde_shadow_ident().unwrap();
            let shadow_var = ctx.shadow_var_ident();
            let mut fields = vec![];
            let input_generics = ctx.input().generics();
            let generics = input_generics.split_for_impl().1;

            from_impl
                .set_span(span)
                .set_for_ident(ctx.input_ident().clone())
                .set_from_generics(Some(input_generics.clone()))
                .set_trait_generics(quote_spanned! {span=> #shadow_ident #generics});

            from_method
                .set_span(span)
                .add_param(quote_spanned! {span=> #shadow_var: #shadow_ident #generics })
                .set_ret_type(quote_spanned! {span=> Self });

            let mut need_default_init = false;
            for fctx in ctx.all_field_ctx() {
                let deserialize = fctx.deserialize();
                if *fctx.serde() && *deserialize {
                    ctx.exec_or_record(|| {
                        let cgen = self.field_codegen(&fctx)?;
                        let field_ident = fctx.ident();
                        let fetch_shadow_field = cgen.field_from_shadow(&fctx)?;
                        let attributes = &fetch_shadow_field.attributes;
                        fields.push(quote_spanned![deserialize.final_span()=>
                            #( #attributes )*
                            #field_ident: #fetch_shadow_field
                        ]);
                        Ok(())
                    });
                }
                else {
                    need_default_init = true;
                }
            }

            // If there are fields that are not deserialized, initialize them with defaults
            let init_from_default = if need_default_init && *arg_props.needs_default() {
                quote_spanned![span=> .. Self::default()]
            }
            else {
                quote![]
            };

            from_method.set_ret_stmt(quote_spanned![span=> Self { #( #fields, )* #init_from_default }]);
            from_impl.add_method(from_method);

            ctx.shadow_struct_mut()?.add_trait_impl(from_impl);
        }

        Ok(())
    }

    fn serde_struct_into_shadow(&'a self) -> darling::Result<()> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let serde = arg_props.serde();
        if *serde && *arg_props.needs_serialize() {
            let span = serde.final_span();
            let mut from_impl = FXImplConstructor::new(format_ident!("From", span = span));
            let mut from_method = FXFnConstructor::new(format_ident!("from", span = span));
            let struct_ident = ctx.input_ident();
            let shadow_ident = arg_props.serde_shadow_ident().unwrap();
            let mut fields = vec![];
            let me_var = ctx.me_var_ident();
            let (_, generics, _) = ctx.input().generics().split_for_impl();

            from_impl
                .set_span(span)
                .set_for_ident(shadow_ident.clone())
                .set_from_generics(Some(ctx.input().generics().clone()))
                .set_trait_generics(quote_spanned! {span=> #struct_ident #generics});

            from_method
                .set_span(span)
                .set_self_ident(me_var)?
                .set_self_type(quote_spanned! {span=> #struct_ident #generics})
                .set_self_borrow(false)
                .set_ret_type(quote_spanned! {span=> Self})
                .set_self_mut(true);

            for fctx in ctx.all_field_ctx() {
                let serialize = fctx.serialize();
                if *fctx.serde() && *serialize {
                    let field_ident = fctx.ident();

                    ctx.exec_or_record(|| {
                        let cgen = self.field_codegen(&fctx)?;
                        let fetch_struct_field = cgen.field_from_struct(&fctx)?;

                        let lazy = fctx.lazy();
                        if *lazy {
                            let lazy_init = cgen.field_lazy_initializer(&fctx, &mut from_method)?;
                            from_method.add_statement(
                                quote_spanned![serialize.final_span()=> #me_var.#field_ident #lazy_init; ],
                            );
                        }

                        fields.push(quote_spanned![serialize.final_span()=> #field_ident: #fetch_struct_field ]);

                        Ok(())
                    });
                }
            }

            from_method.set_ret_stmt(quote_spanned! {span=>
                Self {
                    #( #fields ),*
                }
            });
            from_impl.add_method(from_method);
            ctx.shadow_struct_mut()?.add_trait_impl(from_impl);
        }

        Ok(())
    }

    fn serde_prepare_struct(&'a self) -> darling::Result<()> {
        let ctx = self.ctx();

        if *ctx.arg_props().serde() {
            if let Some(shadow_struct) = self.serde_shadow_struct()? {
                ctx.set_shadow_struct(shadow_struct);

                for fctx in ctx.all_field_ctx() {
                    if *fctx.serde() {
                        match self.field_codegen(&fctx) {
                            Ok(cgen) => {
                                cgen.serde_shadow_field(&fctx)?;
                                cgen.serde_shadow_field_default(&fctx)?;
                            }
                            Err(err) => ctx.push_error(err),
                        }
                    }
                }

                ctx.user_struct_mut()
                    .add_attribute_toks(crate::util::derive_toks(&self.serde_derive_traits()))?;
                ctx.ok_or_record(self.serde_struct_attribute());
            }
        }

        Ok(())
    }

    fn serde_shadow_field_default_fn(&self) -> darling::Result<Option<TokenStream>> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();

        Ok(if let Some(default_value) = arg_props.serde_default_value() {
            let span = default_value.orig().span();

            if default_value.has_value() {
                let shadow_ident = arg_props.serde_shadow_ident().cloned().unwrap();
                let fn_ident = ctx.unique_ident_pfx(&format!("{shadow_ident}_default"));
                let mut default_fn = FXFnConstructor::new_associated(fn_ident.clone());
                let generics = ctx.input().generics().split_for_impl().1;
                let default_span = default_value
                    .orig_span()
                    .unwrap_or_else(|| arg_props.serde().final_span());

                default_fn
                    .set_span(default_span)
                    .set_ret_type(quote_spanned! {default_span=> #shadow_ident #generics})
                    .add_attribute_toks(quote_spanned! {default_span=> #[allow(non_snake_case)] })?;

                let serde_default: TokenStream = if default_value.is_str() {
                    // let default_str: String = (&**default_value).try_into()?;
                    let default_str: String = default_value.try_into()?;
                    let expr: syn::ExprPath = syn::parse_str(&default_str).map_err(|err| {
                        darling::Error::custom(format!("Invalid default string: {err}")).with_span(&span)
                    })?;
                    quote_spanned![default_span=> #expr()]
                }
                else {
                    let default_code = default_value.value().cloned();
                    quote_spanned![default_span=> #default_code]
                };

                // default_fn.set_ret_stmt(quote_spanned! {default_span=> #serde_default.into()});
                default_fn.set_ret_stmt(quote_spanned! {default_span=> #serde_default});
                ctx.add_method(default_fn);
                let default_str = syn::LitStr::new(&format!("{}::{}", ctx.input_ident(), fn_ident), span);

                // return
                Some(quote_spanned![span=> default = #default_str])
            }
            else {
                // return
                Some(quote_spanned![span=> default])
            }
        }
        else {
            None
        })
    }

    fn serde_rewrite_struct(&'a self) {
        let ctx = self.ctx();
        if *ctx.arg_props().serde() {
            ctx.ok_or_record(self.serde_shadow_default_impl());
            ctx.ok_or_record(self.serde_struct_from_shadow());
            ctx.ok_or_record(self.serde_struct_into_shadow());
        }
    }

    fn serde_finalize(&'a self) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        Ok(if *ctx.arg_props().serde() {
            ctx.shadow_struct()?.to_token_stream()
        }
        else {
            quote![]
        })
    }
}

impl<T> FXCGenSerde for T where T: FXCodeGenContextual {}
