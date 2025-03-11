use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};

#[derive(Debug)]
pub(crate) struct FieldConstructor {
    ident:      syn::Ident,
    ty:         TokenStream,
    vis:        Option<TokenStream>,
    attributes: Vec<TokenStream>,
    span:       Span,
}

impl FieldConstructor {
    #[inline]
    pub(crate) fn new<T: ToTokens>(ident: syn::Ident, ty: T, span: Span) -> Self {
        Self {
            ident,
            ty: ty.to_token_stream(),
            vis: None,
            attributes: Vec::new(),
            span,
        }
    }

    #[inline]
    pub(crate) fn add_attribute<T: ToTokens>(&mut self, attribute: T) {
        let attr = attribute.to_token_stream();
        if !attr.is_empty() {
            self.attributes.push(attr);
        }
    }

    #[inline]
    pub(crate) fn add_attributes<T: ToTokens, I: Iterator<Item = T>>(&mut self, attributes: I) {
        for attribute in attributes {
            self.add_attribute(attribute);
        }
    }

    #[inline]
    pub(crate) fn set_vis<T: ToTokens>(&mut self, vis: T) {
        self.vis = Some(vis.to_token_stream());
    }

    #[inline]
    pub(crate) fn set_type<T: ToTokens>(&mut self, ty: T) {
        self.ty = ty.to_token_stream();
    }

    pub(crate) fn to_field(&self) -> TokenStream {
        let vis = self.vis.as_ref();
        let attributes = &self.attributes;
        let ident = &self.ident;
        let ty = &self.ty;

        quote_spanned! {self.span=>
            #(#attributes)*
            #vis #ident: #ty
        }
    }
}

impl ToTokens for FieldConstructor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_field());
    }
}
