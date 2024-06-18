use super::{FXCGenContextual, FXFieldCtx, FXValueRepr};
use crate::helper::with_origin::FXOrig;
use darling::ast::NestedMeta;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, Meta};

pub(crate) trait FXCGenSerde<'f>: FXCGenContextual<'f> {
    // Field is an Option in the shadow struct if it is optional or lazy and has no default value
    fn is_serde_optional(&self, fctx: &FXFieldCtx) -> bool {
        fctx.is_optional() || fctx.is_lazy()
    }

    fn filter_shadow_attributes<'a>(&'a self, fctx: &'a FXFieldCtx) -> impl Iterator<Item = &'a syn::Attribute> {
        // Only use `serde` attribute and those listed in forward_attrs
        let serde_helper = fctx.serde().as_ref().or_else(|| self.ctx().args().serde().as_ref());
        fctx.attrs()
            .iter()
            .filter(move |a| a.path().is_ident("serde") || serde_helper.map_or(false, |sh| sh.accepts_attr(a)))
    }

    fn serde_derive_traits(&self) -> Vec<TokenStream> {
        let mut traits: Vec<TokenStream> = vec![];
        let ctx = self.ctx();
        if ctx.args().is_serde() {
            let serde_arg = ctx.args().serde().as_ref();
            let serde_helper = serde_arg.unwrap();
            let serde_helper_span = serde_helper.to_token_stream().span();

            if serde_helper.needs_serialize().unwrap_or(true) {
                traits.push(quote_spanned![serde_helper_span=> Serialize]);
            }
            if serde_helper.needs_deserialize().unwrap_or(true) {
                traits.push(quote_spanned![serde_helper_span=> Deserialize]);
            }
        }
        return traits;
    }

    fn serde_skip_toks(&self, field_ctx: &FXFieldCtx) -> TokenStream {
        // Don't skip a field if:
        // - no `serde` argument
        // - it is not `serde(off)`
        // - and no more than one of `deserialize` or `serialize` is `off`
        if self.ctx().args().is_serde() {
            let helper_span = field_ctx
                .serde()
                .as_ref()
                .and_then(|sh| sh.orig())
                .map_or(Span::call_site(), |s| s.span());
            if !field_ctx.is_serde() {
                return quote_spanned!(helper_span=> skip );
            }
            if !field_ctx.needs_serialize() {
                return quote_spanned!(helper_span=> skip_serializing );
            }
            if !field_ctx.needs_deserialize() {
                return quote_spanned!(helper_span=> skip_deserializing );
            }
        }
        quote![]
    }

    fn serde_struct_attribute(&self) -> darling::Result<()> {
        let ctx = self.ctx();
        let args = ctx.args();

        ctx.add_attr_from(if args.is_serde() {
            let mut serde_args: Vec<TokenStream> = vec![];

            let serde_helper = args.serde().as_ref().unwrap();
            let generics = ctx.input().generics();
            let shadow_ident = format!("{}{}", ctx.shadow_ident(), generics.to_token_stream());

            if serde_helper.needs_deserialize().unwrap_or(true) {
                serde_args.push(quote![from = #shadow_ident]);
            }
            if serde_helper.needs_serialize().unwrap_or(true) {
                serde_args.push(quote![into = #shadow_ident]);
            }

            if serde_args.len() > 0 {
                quote![ #[serde( #( #serde_args ),*  )] ]
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

    fn serde_field_attribute(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        Ok(if ctx.args().is_serde() {
            let skip_toks = self.serde_skip_toks(fctx);
            let mut serde_attr_args = vec![];

            if !skip_toks.is_empty() {
                serde_attr_args.push(skip_toks);
            }

            let mut default_arg = None;

            if let Some(serde_helper) = fctx.serde().as_ref() {
                if serde_helper.has_default() {
                    // Safe because of has_default()
                    let default_value = serde_helper.default_value_raw().unwrap();
                    let span = default_value.orig().span();

                    if default_value.has_value() {
                        let serde_default_str: String = if default_value.is_str() {
                            default_value.try_into()?
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

                        default_arg = Some(quote_spanned![span=> default = #serde_default_str]);
                    }
                    else {
                        default_arg = Some(quote_spanned![span=> default]);
                    }
                }
            }

            if let Some(default_arg) = default_arg {
                serde_attr_args.push(default_arg);
            }

            if let Some(base_name) = fctx.base_name() {
                let span = base_name.span();
                let base_name = base_name.to_string();
                serde_attr_args.push(quote_spanned![span=> rename = #base_name]);
            }

            if serde_attr_args.is_empty() {
                quote![]
            }
            else {
                quote! [ #[serde( #( #serde_attr_args ),* )] ]
            }
        }
        else {
            quote![]
        })
    }

    fn serde_shadow_field_type(&self, fctx: &FXFieldCtx) -> TokenStream {
        let ty = fctx.ty_tok().clone();
        if self.is_serde_optional(fctx) {
            quote![ ::std::option::Option<#ty> ]
        }
        else {
            quote![ #ty ]
        }
    }

    fn serde_shadow_field_value(&self, fctx: &FXFieldCtx, value: TokenStream) -> TokenStream {
        if self.is_serde_optional(fctx) {
            quote![ ::std::option::Option::Some(#value) ]
        }
        else {
            value
        }
    }

    fn serde_shadow_field(&self, fctx: &FXFieldCtx) {
        let ident = fctx.ident_tok();
        let attrs = self.filter_shadow_attributes(fctx);
        let serde_attr = self.ok_or_empty(self.serde_field_attribute(fctx));
        let user_attrs = fctx
            .serde()
            .as_ref()
            .and_then(|serde_helper| serde_helper.attributes().as_ref())
            .or_else(|| self.ctx().args().attributes().as_ref());
        let ty = self.serde_shadow_field_type(fctx);

        self.add_shadow_field_decl(quote_spanned! [*fctx.span()=>
            #serde_attr
            #user_attrs
            #( #attrs )*
            #ident: #ty
        ]);
    }

    fn serde_shadow_field_default(&self, fctx: &FXFieldCtx) {
        let mut default_tok = self.fixup_self_type(
            self.ok_or_else(self.field_default_value(fctx), || FXValueRepr::None)
                .unwrap_or(quote![::std::default::Default::default()]),
        );

        if self.is_serde_optional(fctx) {
            default_tok = self.serde_shadow_field_value(fctx, default_tok);
        }

        let field_ident = fctx.ident_tok();

        self.add_shadow_default_decl(quote![ #field_ident: #default_tok ]);
    }

    fn serde_shadow_struct(&self) -> darling::Result<()> {
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_serde() {
            let serde_helper = args.serde().as_ref().unwrap();
            let shadow_ident = ctx.shadow_ident();
            let fields = self.shadow_fields();
            let mut attrs = vec![];
            let derive_attr = self.derive_toks(&self.serde_derive_traits());
            let shadow_defaults = self.shadow_defaults();
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;
            let vis = serde_helper.public_mode().map(|pm| pm.to_token_stream());
            let user_attributes = serde_helper.attributes();

            attrs.push(derive_attr);

            if let Some(default_attr_arg) = self.serde_shadow_default_fn()? {
                attrs.push(quote![#[serde(#default_attr_arg)]]);
            }

            ctx.tokens_extend(quote![
                #( #attrs )*
                #user_attributes
                #vis struct #shadow_ident #generics #where_clause {
                    #( #fields ),*
                }

                // #default_impl

                impl #generics ::std::default::Default for #shadow_ident #generics #where_clause {
                    fn default() -> Self {
                        Self {
                            #( #shadow_defaults ),*
                        }
                    }
                }
            ]);
        }

        Ok(())
    }

    // Impl From for the shadow struct
    fn serde_struct_from_shadow(&'f self) {
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_serde() && args.needs_deserialize() {
            let shadow_ident = ctx.shadow_ident();
            let struct_ident = ctx.input_ident();
            let shadow_var = ctx.shadow_var_ident();
            let mut fields = vec![];
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;

            for field in ctx.input().fields() {
                let fctx = self.field_ctx(&field);
                if let Ok(fctx) = fctx {
                    if fctx.is_serde() && fctx.needs_deserialize() {
                        let field_ident = fctx.ident_tok();
                        match self.field_from_shadow(&fctx) {
                            Ok(fetch_shadow_field) => fields.push(quote_spanned![*fctx.span()=>
                                #field_ident: #fetch_shadow_field
                            ]),
                            Err(err) => ctx.push_error(err),
                        }
                    }
                }
                else {
                    ctx.push_error(fctx.unwrap_err())
                }
            }

            ctx.tokens_extend(quote![
                impl #generics ::std::convert::From<#shadow_ident #generics> for #struct_ident #generics #where_clause {
                    fn from(#shadow_var: #shadow_ident #generics) -> Self {
                        Self {
                            #( #fields, )*
                            .. Self::default()
                        }
                    }
                }
            ]);
        }
    }

    fn serde_struct_into_shadow(&'f self) {
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_serde() && args.needs_serialize() {
            let shadow_ident = ctx.shadow_ident();
            let struct_ident = ctx.input_ident();
            let mut fields = vec![];
            let mut lazy_inits = vec![];
            let me_var = ctx.me_var_ident();
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;

            for field in ctx.input().fields() {
                let fctx = self.field_ctx(&field);
                if let Ok(fctx) = fctx {
                    if fctx.is_serde() && fctx.needs_serialize() {
                        let field_ident = fctx.ident_tok();
                        let is_lazy = fctx.is_lazy();
                        let fetch_struct_field = match self.field_from_struct(&fctx) {
                            Ok(f) => f,
                            Err(err) => {
                                ctx.push_error(err);
                                continue;
                            }
                        };

                        if is_lazy {
                            let init_call =
                                self.ok_or_empty(self.field_lazy_initializer(&fctx, Some(quote![(&#me_var)])));
                            lazy_inits.push(quote![ let _ = #init_call; ]);
                        }

                        fields.push(quote_spanned![*fctx.span()=> #field_ident: #fetch_struct_field ]);
                    }
                }
                else {
                    ctx.push_error(fctx.unwrap_err())
                }
            }

            ctx.tokens_extend(quote![
                impl #generics ::std::convert::From<#struct_ident #generics> for #shadow_ident #generics #where_clause {
                    fn from(mut #me_var: #struct_ident #generics) -> Self {
                        #( #lazy_inits )*
                        Self {
                            #( #fields, )*
                        }
                    }
                }
            ])
        }
    }

    fn serde_prepare_struct(&'f self) {
        for field in self.input().fields() {
            let Ok(fctx) = self.field_ctx(field)
            else {
                continue;
            };

            if fctx.is_serde() {
                self.serde_shadow_field(&fctx);
                self.serde_shadow_field_default(&fctx);
            }
        }

        self.ctx().add_attr_from(self.derive_toks(&self.serde_derive_traits()));

        self.ok_or_record(self.serde_struct_attribute());
    }

    fn serde_rewrite_struct(&'f self) {
        self.ok_or_record(self.serde_shadow_struct());
        self.serde_struct_from_shadow();
        self.serde_struct_into_shadow();
    }

    fn serde_shadow_default_fn(&self) -> darling::Result<Option<TokenStream>> {
        let ctx = self.ctx();

        let Some(serde_helper) = ctx.args().serde()
        else {
            return Ok(None);
        };

        if serde_helper.has_default() {
            let default_value = serde_helper.default_value_raw().unwrap();
            let span = default_value.orig().span();

            if default_value.has_value() {
                let serde_default: TokenStream = if default_value.is_str() {
                    let default_str: String = default_value.try_into()?;
                    let expr: syn::ExprPath = syn::parse_str(&default_str).map_err(|err| {
                        darling::Error::custom(format!("Invalid default string: {}", err)).with_span(&span)
                    })?;
                    quote![#expr()]
                }
                else {
                    let default_code = default_value.value().as_ref().unwrap();
                    if let NestedMeta::Meta(Meta::NameValue(_)) = default_code {
                        let err = darling::Error::custom(format!("Unexpected kind of argument")).with_span(&span);
                        #[cfg(feature = "diagnostics")]
                        let err = err.note(format!(
                            "{}\n{}\n{}",
                            "Consider using a string, as with serde `default`: \"Type::function\"`",
                            "                                       or a path: `Type::static_or_constant`",
                            "                       or a call-like expression: `Type::function()`"
                        ));
                        return Err(err);
                    }
                    quote![#default_code]
                };

                let generics = ctx.input().generics();
                let shadow_ident = ctx.shadow_ident();
                let fn_ident = ctx.unique_ident_pfx(&format!("{}_default", shadow_ident.to_string()));
                self.add_method_decl(quote![
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

    fn serde_field_default_fn(&self, fctx: &FXFieldCtx) -> darling::Result<syn::Ident> {
        let fn_ident = fctx.default_fn_ident()?;
        let field_type = self.serde_shadow_field_type(fctx);
        let Some(serde_helper) = fctx.serde()
        else {
            return Err(darling::Error::custom(format!(
                "Can't generate default function for non-serde field {}",
                fctx.ident_str()
            )));
        };
        let Some(serde_default) = serde_helper
            .default_value()
            .map(|dv| self.serde_shadow_field_value(fctx, dv.to_token_stream()))
        else {
            return Err(darling::Error::custom(format!(
                "There is no serde 'default' for field {}",
                fctx.ident_str()
            )));
        };

        self.add_method_decl(quote![
            fn #fn_ident() -> #field_type {
                #serde_default
            }
        ]);
        Ok(fn_ident.clone())
    }
}

impl<'f, T> FXCGenSerde<'f> for T where T: FXCGenContextual<'f> {}
