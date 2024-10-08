use crate::fields::FXField;
use darling::{ast, FromDeriveInput};
use getset::Getters;
use proc_macro2::TokenStream;
use quote::ToTokens;

#[derive(Debug, FromDeriveInput, Getters)]
#[darling(attributes(fieldx), supports(struct_named), forward_attrs)]
#[getset(get = "pub")]
pub struct FXInputReceiver {
    pub vis:      syn::Visibility,
    pub ident:    syn::Ident,
    pub data:     ast::Data<(), FXField>,
    pub attrs:    Vec<syn::Attribute>,
    pub generics: syn::Generics,
}

impl FXInputReceiver {
    pub fn fields(&self) -> Vec<&FXField> {
        self.data.as_ref().take_struct().map_or_else(|| vec![], |s| s.fields)
    }

    pub fn generic_param_idents(&self) -> Vec<TokenStream> {
        let mut idents = vec![];
        for param in self.generics.params.iter() {
            match param {
                syn::GenericParam::Lifetime(ref lf) => idents.push(lf.lifetime.to_token_stream()),
                syn::GenericParam::Type(ref ty) => idents.push(ty.ident.to_token_stream()),
                syn::GenericParam::Const(ref cnst) => idents.push(cnst.ident.to_token_stream()),
            }
        }
        idents
    }
}
