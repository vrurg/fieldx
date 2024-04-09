use darling::{util::Override, FromMeta};
use getset::Getters;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote_spanned, spanned::Spanned, Meta};

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub(crate) struct FXSAttributes {
    list: Vec<syn::Attribute>,
}
#[derive(Debug, Default, Clone, FromMeta, Getters)]
#[getset(get = "pub")]
pub(crate) struct FXSBuilder {
    attributes:      Option<FXSAttributes>,
    attributes_impl: Option<FXSAttributes>,
}

#[derive(Debug, FromMeta, Clone)]
pub(crate) struct FXSArgs {
    sync:    Option<bool>,
    builder: Option<darling::util::Override<FXSBuilder>>,
    into:    Option<bool>,
    // Only plays for sync-safe structs
    no_new:  Option<bool>,
}

impl Default for FXSArgs {
    fn default() -> Self {
        FXSArgs {
            no_new:  Some(false),
            builder: Default::default(),
            sync:    None,
            into:    None,
        }
    }
}

impl FXSArgs {
    pub fn is_sync(&self) -> bool {
        if let Some(ref is_sync) = self.sync {
            *is_sync
        }
        else {
            false
        }
    }

    pub fn needs_new(&self) -> bool {
        if let Some(ref no_new) = self.no_new {
            !*no_new
        }
        else {
            true
        }
    }

    pub fn needs_builder(&self) -> Option<bool> {
        self.builder.as_ref().and(Some(true))
    }

    pub fn needs_into(&self) -> Option<bool> {
        self.into
    }

    pub fn builder_attributes(&self) -> Option<&FXSAttributes> {
        self.builder.as_ref().and_then(|b| {
            if let Override::Explicit(builder) = b {
                builder.attributes.as_ref()
            }
            else {
                None
            }
        })
    }

    pub fn builder_impl_attributes(&self) -> Option<&FXSAttributes> {
        self.builder.as_ref().and_then(|b| {
            if let Override::Explicit(builder) = b {
                builder.attributes_impl.as_ref()
            }
            else {
                None
            }
        })
    }
}

impl FromMeta for FXSAttributes {
    fn from_meta(input: &Meta) -> Result<Self, darling::Error> {
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

impl ToTokens for FXSAttributes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.list.iter().map(|a| a.to_token_stream()));
    }
}
