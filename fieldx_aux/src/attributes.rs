//! `attributes*` family of arguments.

use darling::FromMeta;
use getset::Getters;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote_spanned, spanned::Spanned, Meta};

/// Implementation of `attributes*(...)` family of arguments.
///
/// These are arguments that define bodies of attributes to be applied to certain declarations. I.e.
/// `attributes(derive(Clone), serde(rename_all="lowercase"))` must result in:
///
/// ```ignore
/// #[derive(Clone)]
/// #[serde(rename_all="lowercase")]
/// ```
#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct FXAttributes {
    /// All attribute declarations
    list: Vec<syn::Attribute>,
}

impl IntoIterator for FXAttributes {
    type IntoIter = std::vec::IntoIter<syn::Attribute>;
    type Item = syn::Attribute;

    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
}

impl FromMeta for FXAttributes {
    fn from_meta(input: &Meta) -> Result<Self, darling::Error> {
        // eprintln!(">>> {:#?}", input);
        match input {
            Meta::List(ref ml) => {
                let ml_chunks: Vec<TokenStream> = vec![TokenStream::new()];
                // Split the incoming tokenstream into chunks separated by Puncts (i.e. by commas)
                let mut ml_chunks = ml.tokens.clone().into_iter().fold(ml_chunks, |mut mlc, item| {
                    if let proc_macro2::TokenTree::Punct(_) = item {
                        mlc.push(TokenStream::new());
                    }
                    else {
                        mlc.last_mut().unwrap().extend([item]);
                    }
                    mlc
                });

                // If the list ends with a comma or is empty then we'd end up with an empty chunk at the end.
                if let Some(last_meta) = ml_chunks.last() {
                    if last_meta.is_empty() {
                        let _ = ml_chunks.pop();
                    }
                }

                // Transform chunks into attributes
                let pound = quote![#];
                Ok(Self {
                    list: ml_chunks
                        .iter()
                        .map(|tt| parse_quote_spanned!(tt.span()=> #pound [ #tt ]))
                        .collect(),
                })
            }
            _ => unimplemented!("Can't deal with this kind of input"),
        }
    }
}

impl ToTokens for FXAttributes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.list.iter().map(|a| a.to_token_stream()));
    }
}
