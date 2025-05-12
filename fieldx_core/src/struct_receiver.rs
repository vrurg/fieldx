pub mod args;

use crate::field_receiver::FXField;
use darling::ast;
use darling::FromDeriveInput;
use getset::Getters;

#[derive(Debug, FromDeriveInput, Getters)]
#[darling(attributes(fieldx), supports(struct_named), forward_attrs)]
#[getset(get = "pub")]
pub struct FXStructReceiver {
    pub vis:      syn::Visibility,
    pub ident:    syn::Ident,
    pub data:     ast::Data<(), FXField>,
    pub attrs:    Vec<syn::Attribute>,
    pub generics: syn::Generics,
}

impl FXStructReceiver {
    pub fn fields(&self) -> Vec<&FXField> {
        self.data.as_ref().take_struct().map_or(Vec::new(), |s| s.fields)
    }
}
