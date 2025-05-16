use getset::Getters;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::ToTokens;

use crate::field_receiver::FXField;

use super::FXConstructor;

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct FXFieldConstructor {
    ident:      syn::Ident,
    ty:         TokenStream,
    vis:        Option<TokenStream>,
    attributes: Vec<syn::Attribute>,
    span:       Option<Span>,
}

impl FXFieldConstructor {
    #[inline]
    pub fn new<T: ToTokens>(ident: syn::Ident, ty: T, span: Span) -> Self {
        Self {
            ident,
            ty: ty.to_token_stream(),
            vis: None,
            attributes: Vec::new(),
            span: Some(span),
        }
    }

    #[inline]
    pub fn set_vis<T: ToTokens>(&mut self, vis: T) -> &mut Self {
        self.vis = Some(vis.to_token_stream());
        self
    }

    #[inline]
    pub fn set_type<T: ToTokens>(&mut self, ty: T) -> &mut Self {
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
        #[allow(clippy::redundant_closure)]
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

impl TryFrom<&FXField> for FXFieldConstructor {
    type Error = darling::Error;

    fn try_from(field: &FXField) -> darling::Result<Self> {
        let vis = field.vis().to_token_stream();
        let vis = if vis.is_empty() { None } else { Some(vis) };
        let mut fc = Self {
            ident: field.ident()?,
            ty: field.ty().to_token_stream(),
            attributes: field.attrs().clone(),
            span: Some(field.span()),
            vis,
        };

        fc.add_attributes(field.fieldx_attrs().iter());

        Ok(fc)
    }
}
