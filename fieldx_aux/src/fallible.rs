//! Argument that signals possibility of errors.
use super::FromNestAttr;
use crate::FXProp;
use crate::FXPropBool;
use crate::FXSetState;
use crate::FXSynValue;
use darling::util::Flag;
use darling::FromMeta;
use proc_macro2::Span;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use syn::spanned::Spanned;

/// This argument can be used to mark, say, methods as returning a `Result` and specify what error type is expected.
#[derive(Debug, Clone, FromMeta)]
pub struct FXFallible<T = FXSynValue<syn::Path>>
where
    T: FromMeta,
{
    off:        Flag,
    #[darling(rename = "error")]
    error_type: Option<T>,
}

impl<T> FXFallible<T>
where
    T: FromMeta,
{
    /// Accessor for the error type.
    pub fn error_type(&self) -> Option<&T> {
        self.error_type.as_ref()
    }
}

impl<T> FXSetState for FXFallible<T>
where
    T: FromMeta,
{
    fn is_set(&self) -> FXProp<bool> {
        FXProp::from(self.off).not()
    }
}

impl<T> FromNestAttr for FXFallible<T>
where
    T: FromMeta,
{
    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self {
            off:        Flag::default(),
            error_type: None,
        })
    }
}

impl<T> ToTokens for FXFallible<T>
where
    T: FromMeta + ToTokens,
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut toks = vec![];
        let mut span = self.error_type.as_ref().map_or(Span::call_site(), |et| et.span());
        if self.off.is_present() {
            span = self.off.span();
            toks.push(quote_spanned! {span=> off});
        }
        if let Some(ref error_type) = self.error_type {
            toks.push(quote! { #error_type });
        }
        tokens.extend(quote_spanned! {span=> #(#toks),* });
    }
}
