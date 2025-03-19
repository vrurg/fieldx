pub(crate) mod field;
pub(crate) mod r#fn;
pub(crate) mod r#impl;
pub(crate) mod r#struct;

pub(crate) use field::*;
use fieldx_aux::FXProp;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
pub(crate) use r#fn::*;
pub(crate) use r#impl::*;
pub(crate) use r#struct::*;

macro_rules! tokenstream_setter {
    ( $($name:ident),+ $(,)? ) => {
        $(
            ::paste::paste! {
                #[allow(dead_code)]
pub(crate) fn [<set_ $name>]<T: ToTokens>(&mut self, value: T) -> &mut Self {
                    let tt = value.to_token_stream();
                    self.$name = if tt.is_empty() {
                        None
                    }
                    else {
                        Some(value.to_token_stream())
                    };
                    self
                }
            }
        )+
    }
}

pub(crate) use tokenstream_setter;

use crate::ctx::Attributizer;

pub(crate) trait FXConstructor {
    fn fx_to_tokens(&self) -> TokenStream;
    fn set_span(&mut self, span: proc_macro2::Span) -> &mut Self;
    fn add_attribute(&mut self, attribute: syn::Attribute) -> &mut Self;

    fn split_generics(&self, generics: Option<&syn::Generics>) -> (TokenStream, TokenStream, TokenStream) {
        generics.map_or_else(
            || (quote![], quote![], quote![]),
            |g| {
                let split = g.split_for_impl();
                (
                    split.0.to_token_stream(),
                    split.1.to_token_stream(),
                    split.2.to_token_stream(),
                )
            },
        )
    }

    fn add_attributes<'a, I: Iterator<Item = &'a syn::Attribute>>(&'a mut self, attributes: I) -> &'a mut Self {
        for attribute in attributes {
            self.add_attribute(attribute.clone());
        }
        self
    }

    fn maybe_add_attributes<'a, I: Iterator<Item = &'a syn::Attribute>>(
        &'a mut self,
        attributes: Option<I>,
    ) -> &'a mut Self {
        if let Some(attributes) = attributes {
            self.add_attributes(attributes)
        }
        else {
            self
        }
    }

    fn add_attribute_toks<T>(&mut self, attribute: T) -> darling::Result<&mut Self>
    where
        T: ToTokens,
    {
        let attributes = Attributizer::parse(attribute.to_token_stream())?.into_inner();
        for attribute in attributes {
            self.add_attribute(attribute);
        }
        Ok(self)
    }

    fn add_doc(&mut self, literals: &FXProp<Vec<syn::LitStr>>) -> darling::Result<&mut Self> {
        let lits = literals.value();
        self.add_attributes(
            Attributizer::parse(quote_spanned! {literals.final_span()=>  #( #[doc = #lits] )* })?
                .into_inner()
                .iter(),
        );
        Ok(self)
    }

    #[inline]
    fn maybe_add_doc(&mut self, literals: Option<&FXProp<Vec<syn::LitStr>>>) -> darling::Result<&mut Self> {
        if let Some(literals) = literals {
            self.add_doc(literals)?;
        }
        Ok(self)
    }
}
