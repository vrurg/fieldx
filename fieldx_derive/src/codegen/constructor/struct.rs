use getset::Getters;
use getset::MutGetters;
use getset::Setters;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::ToTokens;

use super::field::FXFieldConstructor;
use super::tokenstream_setter;
use super::FXConstructor;
use super::FXImplConstructor;

#[derive(Debug, Getters, MutGetters, Setters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXStructConstructor {
    ident:       syn::Ident,
    vis:         Option<TokenStream>,
    attributes:  Vec<syn::Attribute>,
    generics:    Option<syn::Generics>,
    fields:      Vec<FXFieldConstructor>,
    #[getset(get_mut = "pub(crate)")]
    struct_impl: FXImplConstructor,
    trait_impls: Vec<FXImplConstructor>,
    span:        Option<Span>,
}

impl FXStructConstructor {
    tokenstream_setter! { vis}

    pub(crate) fn new(ident: syn::Ident) -> Self {
        Self {
            struct_impl: FXImplConstructor::new(ident.clone()),
            ident,
            vis: None,
            attributes: Vec::new(),
            generics: None,
            fields: Vec::new(),
            trait_impls: Vec::new(),
            span: None,
        }
    }

    pub(crate) fn set_generics(&mut self, generics: syn::Generics) -> &mut Self {
        self.generics = Some(generics.clone());
        self.struct_impl_mut().set_from_generics(Some(generics));
        self
    }

    pub(crate) fn add_field(&mut self, field: FXFieldConstructor) -> &mut Self {
        self.fields.push(field);
        self
    }

    pub(crate) fn add_trait_impl(&mut self, trait_impl: FXImplConstructor) -> &mut Self {
        self.trait_impls.push(trait_impl);
        self
    }

    pub(crate) fn field_idents(&self) -> impl Iterator<Item = &syn::Ident> {
        self.fields.iter().map(|field| field.ident())
    }

    // Re-order attributes in a way that `derive` attribute comes first.
    pub(crate) fn ordered_attrs(&self) -> Vec<syn::Attribute> {
        let mut attrs = self.attributes.clone();
        attrs.sort_by(|a, b| {
            if a.path().is_ident("derive") && !b.path().is_ident("derive") {
                std::cmp::Ordering::Less
            }
            else if !a.path().is_ident("derive") && b.path().is_ident("derive") {
                std::cmp::Ordering::Greater
            }
            else {
                std::cmp::Ordering::Equal
            }
        });
        attrs
    }
}

impl FXConstructor for FXStructConstructor {
    fn fx_to_tokens(&self) -> TokenStream {
        let vis = self.vis.as_ref();
        let span = self.span.unwrap_or_else(|| Span::call_site());
        let attributes = self.ordered_attrs();
        let ident = &self.ident;
        let generics = self.generics.as_ref();
        let where_clause = generics.map(|g| &g.where_clause);
        let fields = &self.fields;
        let struct_impl = &self.struct_impl;
        let trait_impls = &self.trait_impls;

        quote_spanned! {span=>
            #(#attributes)*
            #vis struct #ident #generics #where_clause {
                #(#fields),*
            }

            #struct_impl
            #( #trait_impls )*
        }
    }

    #[inline(always)]
    fn set_span(&mut self, span: proc_macro2::Span) -> &mut Self {
        self.span = Some(span);
        self.struct_impl_mut().set_from_span(Some(span));
        self
    }

    #[inline(always)]
    fn add_attribute(&mut self, attribute: syn::Attribute) -> &mut Self {
        self.attributes.push(attribute);
        self
    }
}

impl ToTokens for FXStructConstructor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.fx_to_tokens());
    }
}
