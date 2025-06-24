#![doc(html_root_url = "https://docs.rs/fieldx_derive_support/")]
//! # fieldx_derive_support
//!
//! This crate provides automations to simplify development of the `fieldx_aux` and `fieldx_derive` crates.

use std::collections::HashMap;

use darling::ast;
use darling::util::Flag;
use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use syn::parse::Parse;
use syn::parse_macro_input;
use syn::parse_quote;
use syn::parse_quote_spanned;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::DeriveInput;

#[derive(Debug, FromMeta, Clone)]
struct FXHArgs {
    validate:  Option<syn::Path>,
    to_tokens: Flag,
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

/// This macro is used to generate helper structs. It adds the following fields to support related helper
/// functionality:
///
/// - **`off`** - a [`Flag`] that indicates if the helper is disabled.
/// - **`name`** - an optional name for the helper. Particular use depends on the helper semantics.
/// - **`attributes_fn`** - list of attributes to apply to the routines, generated in support of the helper-specified functionality.
/// - **`visibility`** - the visibility of the helper-generated syntax elements. It is user-facing as `vis` helper argument.
/// - **`private`** - a shortcut for `vis()` with `syn::Visibility::Inherited` what usually means "private".
/// - **`doc`** - provides a `doc` argument to add documentation to the helper.
///
/// The macro takes the following arguments:
///
/// - **`validate(<path>)`** - a function name to be called to validate the helper. It is expected to return `darling::Result<()>`.
/// - **`to_tokens`** - if set then the helper will get auto-generated implementation of the `ToTokens` trait.
///
/// A field-level attribute of the same name `fxhelper` is provided with the following arguments:
///
/// - **`exclusive("exclusive group")`** - if two or more fields of the helper struct are mutually exclusive then then must
///   be marked with the same group name. This will cause a compile-time error if more than one of them is set.
/// - **`rename("alias")`** - an alias for the field. This is used to generate the user-facing argument name.
///
/// For example:
///
/// ```ignore
/// #[fxhelper(validate = Self::validate, to_tokens)]
/// struct MyHelper {
///     #[fxhelper(rename = "mutable", exclusive = "use mode")]
///     is_mutable: Option<FXBool>,
///     #[fxhelper(rename = "inner_mut", exclusive = "use mode")]
///     is_inner_mut: Option<FXBool>,
///
///     #[fxhelper(exclusive = "fubar")]
///     foo: Option<FXString>,
///     #[fxhelper(exclusive = "fubar")]
///     bar: Option<FXString>,
/// }
/// ```
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
        Err(e) => return e.write_errors().into(),
    };

    let incopy = input.clone();
    let input_ast = parse_macro_input!(incopy as DeriveInput);
    let fx = match FXHelperStruct::from_derive_input(&input_ast) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
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
            ("vis".to_string(), format_ident!("visibility"), quote![is_set_bool()]),
            ("private".to_string(), format_ident!("private"), quote![is_set_bool()]),
        ],
    );

    // List of token stream producers for ToTokens implementation
    let mut metas = ["name", "attributes_fn", "visibility", "private", "doc"]
        .iter()
        .map(|s| format_ident!("{}", s))
        .collect::<Vec<_>>();

    for field in fields.iter() {
        let mut custom_attrs = vec![];
        let field_alias = field.rename.as_ref().cloned();

        if let Some(ref alias) = field_alias {
            custom_attrs.push(quote![ #[darling(rename = #alias)] ]);
        }

        if let Some(ref field_ident) = field.ident {
            metas.push(field_ident.clone());
        }

        if let Some(exclusive) = &field.exclusive {
            if let Some(ref ident) = field.ident {
                let ident = ident.clone();

                let check_method = if let syn::Type::Path(ref tpath) = field.ty {
                    let span = tpath.span();
                    if tpath.path.is_ident("Flag") {
                        quote_spanned![span=> is_present()]
                    }
                    else {
                        quote_spanned![span=> is_set_bool()]
                    }
                }
                else {
                    return darling::Error::unexpected_type(&field.ty.to_token_stream().to_string())
                        .write_errors()
                        .into();
                };

                exclusives.entry(exclusive.clone()).or_default().push((
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
        .any(|f| f.ident.as_ref().map_or("".to_string(), |i| i.to_string()) == "attributes")
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
                if self.#ident. #check_method {
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

    let pound = syn::Token![#](Span::call_site());

    let to_tokens_impl = if args.to_tokens.is_present() {
        let span = args.to_tokens.span();
        quote_spanned! {span=>
            impl #impl_generics ::quote::ToTokens for #ident #ty_generics #where_clause {
                fn to_tokens(&self, tokens: &mut TokenStream) {
                    let mut parts = vec![];
                    if self.off.is_present() {
                        parts.push(::quote::quote! { off });
                    }
                    #(
                        let meta_toks = self.#metas.to_token_stream();
                        if !meta_toks.is_empty() {
                            parts.push(meta_toks);
                        }
                    )*

                    tokens.extend(::quote::quote! { #pound(#pound parts),* });
                }
            }
        }
    }
    else {
        quote! {}
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

        impl #impl_generics FXSetState for #ident #ty_generics #where_clause {
            #[inline]
            fn is_set(&self) -> FXProp<bool> {
                FXProp::from(self.off).not()
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
                if self.private.as_ref().map_or(false, |v| *v.is_set()) {
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

        #to_tokens_impl
    ]
    .into()
}

#[derive(Debug)]
enum FallbackParam {
    Or(syn::Expr),
    Else(syn::Block),
    Clone(Span),
    AsRef(Span),
}

impl FallbackParam {
    fn idx(&self) -> usize {
        match self {
            FallbackParam::Or(_) | FallbackParam::Else(_) => 0,
            FallbackParam::Clone(_) => 1,
            FallbackParam::AsRef(_) => 2,
        }
    }

    fn kwd_for_idx(idx: usize) -> &'static str {
        match idx {
            0 => "default",
            1 => "cloned",
            2 => "as_ref",
            _ => panic!("Invalid index"),
        }
    }

    fn span(&self) -> Span {
        match self {
            FallbackParam::Or(expr) => expr.span(),
            FallbackParam::Else(block) => block.span(),
            FallbackParam::Clone(span) => *span,
            FallbackParam::AsRef(span) => *span,
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
                "as_ref" => Ok(Self::AsRef(ident.span())),
                _ => Err(input.error("Expected `default`, `cloned`, or `as_ref` keyword")),
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
    is_ref:      bool,
    ref_span:    Option<proc_macro2::Span>,
    params:      Vec<FallbackParam>,
    span:        proc_macro2::Span,
    /// If this property is final its return type is `FXProp<T>` and never `Option<T>`. For now the final status is
    /// defined by the implicity of the bool return type.
    is_final:    bool,
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
                is_ref: false,
                ref_span: None,
                params: vec![FallbackParam::Else(block)],
                span,
                is_final: true,
            });
        }

        let mut ref_span = None;
        let is_ref = if input.peek(syn::Token![&]) {
            let t: syn::Token![&] = input.parse()?;
            ref_span = Some(t.span);
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
                return Err(syn::Error::new(param.span(), format!("Multiple `{kwd}` parameters")));
            }
            params.push(param);
        }

        Ok(Self {
            method_name,
            return_type,
            is_ref,
            ref_span,
            params,
            span,
            is_final: false,
        })
    }
}

impl ToTokens for FallbackArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;
        let prop_name = &self.method_name;
        let mut return_type = self.return_type.to_token_stream();
        let mut or_else = None;
        let mut clone = None;
        let mut as_ref = None;

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
                FallbackParam::AsRef(_) => {
                    as_ref = Some(quote_spanned! {param_span=> .as_ref()});
                }
            }
        }

        let (ref_sym, deref) = if self.is_ref {
            (quote_spanned![self.ref_span.unwrap_or(span)=> &], quote![])
        }
        else if as_ref.is_some() {
            (quote![], quote![])
        }
        else {
            (quote![], quote_spanned![span=> *])
        };

        if self.is_final {
            return_type = quote_spanned! {return_type.span()=> FXProp<#return_type> };
        }
        return_type = quote_spanned! {return_type.span()=> #ref_sym #return_type };

        let tt = quote_spanned! {span=>
            #[inline]
            pub fn #prop_name(&self) -> #return_type {
                let field_props = self.field_props();
                let arg_props = self.arg_props();
                #deref self.#prop_name
                    .get_or_init(|| {
                        field_props
                            .#prop_name()
                            #clone
                            .or_else(|| arg_props.#prop_name() #clone)
                            #or_else
                    }) #as_ref
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
