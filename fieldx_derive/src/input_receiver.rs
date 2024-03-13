use crate::fields::FXField;
use darling::{ast, FromDeriveInput};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(fieldx), supports(struct_named), forward_attrs)]
pub struct FXInputReceiver {
    pub vis:   syn::Visibility,
    pub ident: syn::Ident,
    pub data:  ast::Data<(), FXField>,
    pub attrs: Vec<syn::Attribute>,
}

impl FXInputReceiver {
    pub fn fields(&self) -> Vec<&FXField> {
        self.data.as_ref().take_struct().unwrap().fields
    }
}

// impl ToTokens for FXInputReceiver {
//     fn to_tokens(&self, tokens: &mut TokenStream) {
//         tokens.extend(self.rewrite());
//     }
// }
