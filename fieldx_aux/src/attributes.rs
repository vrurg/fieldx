//! `attributes*` family of arguments.

use std::borrow::Borrow;
use std::ops::Deref;

use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::ToTokens;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_quote_spanned;
use syn::spanned::Spanned;

/// Implementation of a single sub-argument of `attributes*(...)` family of arguments.
///
/// These are arguments that define bodies of attributes to be applied to certain declarations. I.e.
/// `attributes(derive(Debug, Clone), serde(rename_all="lowercase"))` must result in:
///
/// ```ignore
/// #[derive(Debug, Clone)]
/// #[serde(rename_all="lowercase")]
/// ```
///
/// This struct is responsible for parsing [metalist](https://docs.rs/syn/latest/syn/struct.MetaList.html) elements like
/// `derive(Debug, Clone)` or `serde(rename_all="lowercase")` and converting them into `syn::Attribute` instances.
#[derive(Debug, Clone)]
pub struct FXAttribute(syn::Attribute);

impl Parse for FXAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attr_meta: syn::MetaList = input.parse()?;
        let span = attr_meta.span();
        let pound = quote_spanned![span=> #];
        let attr: syn::Attribute = parse_quote_spanned! {span=> #pound [ #attr_meta ] };
        Ok(FXAttribute(attr))
    }
}

impl From<FXAttribute> for syn::Attribute {
    fn from(attr: FXAttribute) -> Self {
        attr.0
    }
}

impl<'a> From<&'a FXAttribute> for &'a syn::Attribute {
    fn from(attr: &'a FXAttribute) -> Self {
        &attr.0
    }
}

impl Deref for FXAttribute {
    type Target = syn::Attribute;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<syn::Attribute> for FXAttribute {
    fn as_ref(&self) -> &syn::Attribute {
        &self.0
    }
}

impl Borrow<syn::Attribute> for FXAttribute {
    fn borrow(&self) -> &syn::Attribute {
        &self.0
    }
}

impl ToTokens for FXAttribute {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
