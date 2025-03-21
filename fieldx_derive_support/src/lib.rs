use std::collections::HashMap;

use darling::{ast, FromDeriveInput, FromField, FromMeta};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned,
    token::Comma, DeriveInput,
};

#[derive(Debug, FromMeta, Clone)]
struct FXHArgs {
    validate: Option<syn::Path>,
}

#[derive(Debug, FromField, Clone)]
#[darling(attributes(fxhelper), forward_attrs)]
struct FXHelperField {
    ident: Option<syn::Ident>,
    vis:   syn::Visibility,
    ty:    syn::Type,
    attrs: Vec<syn::Attribute>,

    exclusive: Option<String>,
    rename:    Option<String>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs)]
struct FXHelperStruct {
    vis:      syn::Visibility,
    ident:    syn::Ident,
    data:     ast::Data<(), FXHelperField>,
    attrs:    Vec<syn::Attribute>,
    generics: syn::Generics,
}

#[proc_macro_attribute]
pub fn fxhelper(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr_args = match ast::NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return darling::Error::from(e).write_errors().into();
        }
    };

    let args = match FXHArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    let incopy = input.clone();
    let input_ast = parse_macro_input!(incopy as DeriveInput);
    let fx = match FXHelperStruct::from_derive_input(&input_ast) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    let FXHelperStruct {
        vis,
        ident,
        data,
        attrs,
        generics,
    } = &fx;

    let ast::Data::Struct(fields) = data
    else {
        panic!("Expected struct data")
    };

    let mut fields_tt: Vec<TokenStream> = Vec::new();
    let mut exclusives: HashMap<String, Vec<(String, syn::Ident, TokenStream)>> = HashMap::new();
    let mut exclusives_tt: Vec<TokenStream> = vec![];

    exclusives.insert(
        "visibility".to_string(),
        vec![
            ("vis".to_string(), format_ident!("visibility"), quote![is_some]),
            ("private".to_string(), format_ident!("private"), quote![is_some]),
        ],
    );

    for field in fields.iter() {
        let mut custom_attrs = vec![];
        let field_alias = field.rename.as_ref().cloned();

        if let Some(ref alias) = field_alias {
            custom_attrs.push(quote![ #[darling(rename = #alias)] ]);
        }

        if let Some(exclusive) = &field.exclusive {
            if let Some(ref ident) = field.ident {
                let ident = ident.clone();

                let check_method = if let syn::Type::Path(ref tpath) = field.ty {
                    if tpath.path.is_ident("Flag") {
                        quote![is_present]
                    }
                    else {
                        quote![is_some]
                    }
                }
                else {
                    return darling::Error::unexpected_type(&field.ty.to_token_stream().to_string())
                        .write_errors()
                        .into();
                };

                exclusives.entry(exclusive.clone()).or_insert(vec![]).push((
                    if let Some(alias) = field_alias {
                        alias
                    }
                    else {
                        ident.to_string()
                    },
                    ident,
                    check_method,
                ));
            }
        }

        let FXHelperField {
            ident, vis, ty, attrs, ..
        } = &field;

        fields_tt.push(quote![ #( #attrs )* #( #custom_attrs )* #vis #ident: #ty ])
    }

    let attributes_method = if fields
        .iter()
        .find(|f| (*f).ident.as_ref().map_or("".to_string(), |i| i.to_string()) == "attributes")
        .is_some()
    {
        quote![
            fn attributes(&self) -> Option<&FXAttributes> {
                self.attributes.as_ref()
            }
        ]
    }
    else {
        quote![
            fn attributes(&self) -> Option<&FXAttributes> {
                None
            }
        ]
    };

    let mut getters_derive = quote![ #[derive(Getters)] ];
    for attr in attrs {
        if attr.path().is_ident("derive") {
            let args = attr
                .parse_args_with(Punctuated::<syn::Path, Comma>::parse_terminated)
                .unwrap();
            if args.iter().any(|a| a.is_ident("Getters")) {
                getters_derive = quote![];
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let getset_vis = vis.to_token_stream().to_string();

    for (group, fields) in exclusives.iter() {
        let mut checks: Vec<TokenStream> = vec![];

        for (field_name, ident, check_method) in fields.iter() {
            checks.push(quote![
                if self.#ident.#check_method() {
                    set_params.push(#field_name);
                }
            ]);
        }

        exclusives_tt.push(quote![
            {
                let mut set_params: Vec<&str> = vec![];
                #(#checks)*

                if set_params.len() > 1 {
                    let err = darling::Error::custom(
                        format!(
                            "The following options from group '{}' cannot be used together: {}",
                            #group,
                            set_params.iter().map(|f| format!("`{}`", f)).collect::<Vec<String>>().join(", ") ));
                    return Err(err);
                }
            }
        ]);
    }

    let self_validate = if let Some(validate_name) = args.validate {
        let span = validate_name.span();
        quote_spanned! {
            span=>
            #validate_name(&self)?;
        }
    }
    else {
        quote![]
    };

    quote! [
        #[derive(FromMeta, Clone)]
        #( #attrs )*
        #[darling(and_then = Self::__validate_helper)]
        #getters_derive
        #vis struct #ident #generics #where_clause {
            #[getset(skip)]
            name:        Option<FXString>,
            /// If true then helper is disabled.
            #[getset(get = #getset_vis)]
            off:           Flag,
            #[getset(skip)]
            attributes_fn: Option<FXAttributes>,
            #[getset(skip)]
            #[darling(rename = "vis")]
            visibility: Option<crate::FXSynValue<syn::Visibility>>,
            private: Option<FXBool>,
            doc: Option<crate::FXDoc>,

            #( #fields_tt ),*
        }

        impl #impl_generics FXTriggerHelper for #ident #ty_generics #where_clause {
            fn is_true(&self) -> FXProp<bool> {
                let is_present = self.off.is_present();
                FXProp::new(
                    !is_present,
                    if is_present { Some(self.off.span()) } else { None }
                )
            }
        }

        impl #impl_generics FXSetState for #ident #ty_generics #where_clause {
            fn is_set(&self) -> FXProp<bool> {
                if self.off.is_present() {
                    FXProp::new(false, Some(self.off.span()))
                }
                else {
                    FXProp::new(true, None)
                }
            }
        }

        impl #impl_generics crate::FXHelperTrait for #ident #ty_generics #where_clause {
            #[inline]
            fn name(&self) -> Option<FXProp<&str>> {
                if let Some(ref name) = self.name {
                    name.value().map(|v| FXProp::new(v.as_str(), name.orig_span()))
                }
                else {
                    None
                }
            }

            #[inline]
            fn attributes_fn(&self) -> Option<&FXAttributes> {
                self.attributes_fn.as_ref()
            }

            #[inline]
            fn visibility(&self) -> Option<&syn::Visibility> {
                if self.private.as_ref().map_or(false, |v| *v.is_true()) {
                    return Some(&syn::Visibility::Inherited);
                }
                self.visibility.as_ref().map(|v| v.value())
            }

            #[inline]
            fn doc(&self) -> Option<&crate::FXDoc> {
                self.doc.as_ref()
            }

            #attributes_method
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            fn validate_exclusives(&self) -> ::darling::Result<()> {
                #(#exclusives_tt)*
                Ok(())
            }

            fn __validate_helper(self) -> ::darling::Result<Self> {
                self.validate_exclusives()?;
                #self_validate
                Ok(self)
            }
        }
    ]
    .into()
}

#[derive(Debug)]
enum FallbackParam {
    Or(syn::Expr),
    Else(syn::Block),
    Clone(Span),
}

impl FallbackParam {
    fn idx(&self) -> usize {
        match self {
            FallbackParam::Or(_) | FallbackParam::Else(_) => 0,
            FallbackParam::Clone(_) => 1,
        }
    }

    fn kwd_for_idx(idx: usize) -> &'static str {
        match idx {
            0 => "default",
            1 => "cloned",
            _ => panic!("Invalid index"),
        }
    }

    fn span(&self) -> Span {
        match self {
            FallbackParam::Or(expr) => expr.span(),
            FallbackParam::Else(block) => block.span(),
            FallbackParam::Clone(span) => *span,
        }
    }
}

impl Parse for FallbackParam {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "default" => {
                    if input.peek(syn::token::Brace) {
                        let block = input.parse::<syn::Block>()?;
                        Ok(Self::Else(block))
                    }
                    else {
                        let expr = input.parse::<syn::Expr>()?;
                        Ok(Self::Or(expr))
                    }
                }
                "cloned" => Ok(Self::Clone(ident.span())),
                _ => Err(input.error("Expected `default` or `cloned`")),
            }
        }
        else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug)]
struct FallbackArg {
    method_name: syn::Ident,
    return_type: syn::Type,
    as_ref:      bool,
    as_ref_span: Option<proc_macro2::Span>,
    params:      Vec<FallbackParam>,
    span:        proc_macro2::Span,
}

impl Parse for FallbackArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let method_name = input.parse::<syn::Ident>()?;
        let _: syn::Token![,] = input.parse()?;

        // Short for for boolean: specify `true` or `false` as the default value instead of the return type.
        if input.peek(syn::LitBool) {
            let ident: syn::LitBool = input.parse()?;
            let block: syn::Block = parse_quote_spanned! {ident.span()=>
                {
                    FXProp::new(#ident, *field_props.field().fieldx_attr_span())
                }
            };
            return Ok(Self {
                method_name,
                return_type: parse_quote! { bool },
                as_ref: false,
                as_ref_span: None,
                params: vec![FallbackParam::Else(block)],
                span,
            });
        }

        let mut as_ref_span = None;
        let as_ref = if input.peek(syn::Token![&]) {
            let t: syn::Token![&] = input.parse()?;
            as_ref_span = Some(t.span);
            true
        }
        else {
            false
        };

        let return_type = input.parse::<syn::Type>()?;

        let mut params = vec![];
        let mut param_count = HashMap::<usize, usize>::new();

        while input.peek(syn::Token![,]) {
            let _ = input.parse::<syn::Token![,]>();
            let param = input.parse::<FallbackParam>()?;
            if *param_count.entry(param.idx()).or_insert(0) > 1 {
                let kwd = FallbackParam::kwd_for_idx(param.idx());
                return Err(syn::Error::new(param.span(), format!("Multiple `{}` parameters", kwd)));
            }
            params.push(param);
        }

        Ok(Self {
            method_name,
            return_type,
            as_ref,
            as_ref_span,
            params,
            span,
        })
    }
}

impl ToTokens for FallbackArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;
        let prop_name = &self.method_name;
        let return_type = &self.return_type;
        let (as_ref, deref) = if self.as_ref {
            (quote_spanned![self.as_ref_span.unwrap_or(span)=> &], quote![])
        }
        else {
            (quote![], quote_spanned![span=> *])
        };
        let mut or_else = None;
        let mut clone = None;

        for param in self.params.iter() {
            let param_span = param.span();
            match param {
                FallbackParam::Or(expr) => {
                    or_else = Some(quote_spanned! {param_span=> .unwrap_or(#expr)});
                }
                FallbackParam::Else(expr) => {
                    or_else = Some(quote_spanned! {param_span=> .unwrap_or_else(|| #expr)});
                }
                FallbackParam::Clone(_) => {
                    clone = Some(quote_spanned! {param_span=> .cloned()});
                }
            }
        }

        let ret_type = quote_spanned! {return_type.span()=> #as_ref FXProp<#return_type> };

        let tt = quote_spanned! {span=>
            #[inline]
            pub fn #prop_name(&self) -> #ret_type {
                let field_props = self.field_props();
                let arg_props = self.arg_props();
                #deref self.#prop_name
                    .get_or_init(|| {
                        field_props
                            .#prop_name()
                            #clone
                            .or_else(|| arg_props.#prop_name() #clone)
                            #or_else
                    })
            }
        };
        tokens.extend(tt);
    }
}

struct FallbackArgList {
    args: Vec<FallbackArg>,
}

impl Parse for FallbackArgList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<FallbackArg, syn::Token![;]>::parse_terminated(input)?;

        Ok(Self {
            args: args.into_iter().collect(),
        })
    }
}

impl ToTokens for FallbackArgList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let args = &self.args;
        tokens.extend(quote! {
            #( #args )*
        });
    }
}

// This macro is specifically designed for FieldCTXProps struct
#[proc_macro]
pub fn fallback_prop(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args: FallbackArgList = parse_macro_input!(input as FallbackArgList);

    let toks = args.to_token_stream();

    toks.into()
}
