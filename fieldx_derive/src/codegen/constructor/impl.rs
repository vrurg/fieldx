use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};

use super::{tokenstream_setter, FXConstructor, FXFnConstructor};

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXImplConstructor {
    ident:          syn::Ident,
    /// The struct for which this impl block is being generated. If defined then the ident field stands for the trait
    /// name.
    for_ident:      Option<TokenStream>,
    attributes:     Vec<syn::Attribute>,
    impl_generics:  Option<TokenStream>,
    trait_generics: Option<TokenStream>,
    generics:       Option<TokenStream>,
    where_clause:   Option<TokenStream>,
    methods:        Vec<FXFnConstructor>,
    span:           Option<Span>,
}

impl FXImplConstructor {
    tokenstream_setter! { impl_generics, trait_generics, generics, where_clause }

    pub(crate) fn new(ident: syn::Ident) -> Self {
        Self {
            ident,
            for_ident: None,
            attributes: Vec::new(),
            impl_generics: None,
            trait_generics: None,
            generics: None,
            where_clause: None,
            methods: Vec::new(),
            span: None,
        }
    }

    pub(crate) fn set_from_generics(&mut self, generics: Option<syn::Generics>) -> &mut Self {
        if let Some(generics) = generics {
            let (impl_generics, generics, where_clause) = self.split_generics(Some(&generics));
            self.generics = Some(generics);
            self.impl_generics = Some(impl_generics);
            self.where_clause = Some(where_clause);
        }
        else {
            self.generics = None;
            self.impl_generics = None;
            self.where_clause = None;
        }
        self
    }

    pub(crate) fn set_from_span(&mut self, span: Option<Span>) -> &mut Self {
        if span.is_some() {
            self.span = span;
        }
        self
    }

    pub(crate) fn set_for_ident<T: ToTokens>(&mut self, for_ident: T) -> &mut Self {
        self.for_ident = Some(for_ident.to_token_stream());
        self
    }

    pub(crate) fn add_method(&mut self, method: FXFnConstructor) -> &mut Self {
        self.methods.push(method);
        self
    }
}

impl FXConstructor for FXImplConstructor {
    fn fx_to_tokens(&self) -> TokenStream {
        let span = self.span.unwrap_or_else(|| Span::call_site());
        let attributes = &self.attributes;
        let ident = &self.ident;
        let impl_generics = self.impl_generics.as_ref();
        let generics = self.generics.as_ref();
        let where_clause = self.where_clause.as_ref();
        let methods = &self.methods;
        let for_ident = self.for_ident.as_ref().map(|fi| quote_spanned! {span=> for #fi });
        let trait_generics_and_for_ident = self
            .trait_generics
            .as_ref()
            .map(|tg| {
                let tg = if tg.is_empty() {
                    quote![]
                }
                else {
                    quote_spanned! {span=> <#tg> }
                };
                quote_spanned! {span=> #tg #for_ident }
            })
            .or(for_ident);

        quote_spanned! {span=>
            #(#attributes)*
            impl #impl_generics #ident #trait_generics_and_for_ident #generics #where_clause {
                #(#methods)*
            }
        }
    }

    fn set_span(&mut self, span: proc_macro2::Span) -> &mut Self {
        self.span = Some(span);
        self
    }

    fn add_attribute(&mut self, attribute: syn::Attribute) -> &mut Self {
        self.attributes.push(attribute);
        self
    }
}

impl ToTokens for FXImplConstructor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.fx_to_tokens());
    }
}
