use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};

use super::FXConstructor;

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXFieldConstructor {
    ident:      syn::Ident,
    ty:         TokenStream,
    vis:        Option<TokenStream>,
    attributes: Vec<syn::Attribute>,
    span:       Option<Span>,
}

impl FXFieldConstructor {
    #[inline]
    pub(crate) fn new<T: ToTokens>(ident: syn::Ident, ty: T, span: Span) -> Self {
        Self {
            ident,
            ty: ty.to_token_stream(),
            vis: None,
            attributes: Vec::new(),
            span: Some(span),
        }
    }

    #[inline]
    pub(crate) fn set_vis<T: ToTokens>(&mut self, vis: T) -> &mut Self {
        self.vis = Some(vis.to_token_stream());
        self
    }

    #[inline]
    pub(crate) fn set_type<T: ToTokens>(&mut self, ty: T) -> &mut Self {
        self.ty = ty.to_token_stream();
        self
    }
}

impl FXConstructor for FXFieldConstructor {
    fn fx_to_tokens(&self) -> TokenStream {
        let vis = self.vis.as_ref();
        let attributes = &self.attributes;
        let ident = &self.ident;
        let ty = &self.ty;
        let span = self.span.unwrap_or_else(|| Span::call_site());

        quote_spanned! {span=>
            #(#attributes)*
            #vis #ident: #ty
        }
    }

    fn set_span(&mut self, span: proc_macro2::Span) -> &mut Self {
        self.span = Some(span);
        self
    }

    #[inline]
    fn add_attribute(&mut self, attribute: syn::Attribute) -> &mut Self {
        self.attributes.push(attribute);
        self
    }
}

impl ToTokens for FXFieldConstructor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.fx_to_tokens());
    }
}
