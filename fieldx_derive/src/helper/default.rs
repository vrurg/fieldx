use super::{FXOrig, FXTriggerHelper};
use darling::{ast::NestedMeta, FromMeta};
use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{spanned::Spanned, Meta};

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXDefault<const OPTIONAL: bool = false> {
    #[getset(skip)]
    off:   bool,
    value: Option<NestedMeta>,
    orig:  Option<TokenStream>,
}

impl<const OPTIONAL: bool> FXDefault<OPTIONAL> {
    #[allow(dead_code)]
    pub fn is_str(&self) -> bool {
        if let Some(NestedMeta::Lit(syn::Lit::Str(ref _lit))) = self.value {
            true
        }
        else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }
}

impl<const OPTIONAL: bool> FromMeta for FXDefault<OPTIONAL> {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let mut off = false;
        let meta = if items.len() == 2 {
            match &items[0] {
                NestedMeta::Meta(Meta::Path(path)) if path.is_ident("off") && path.segments.len() == 1 => {
                    off = true;
                }
                _ => return Err(darling::Error::custom("The first argument can only be 'off' keyword")),
            }
            Some(items[1].clone())
        }
        else if items.len() == 0 {
            if OPTIONAL {
                None
            }
            else {
                return Err(darling::Error::too_few_items(1));
            }
        }
        else if items.len() > 2 {
            return Err(darling::Error::too_many_items(2));
        }
        else {
            Some(items[0].clone())
        };

        let mut orig = TokenStream::new();
        orig.extend(items.iter().map(|i| i.to_token_stream()));

        Ok(Self {
            off,
            value: meta,
            orig: Some(orig),
        })
    }

    fn from_word() -> darling::Result<Self> {
        if OPTIONAL {
            Ok(Self {
                off:   Default::default(),
                value: None,
                orig:  None,
            })
        }
        else {
            Err(darling::Error::custom("The actual default value is required"))
        }
    }
}

impl<const OPTIONAL: bool> FXTriggerHelper for FXDefault<OPTIONAL> {
    fn is_true(&self) -> bool {
        !self.off
    }
}

impl<const OPTIONAL: bool> FXOrig<TokenStream> for FXDefault<OPTIONAL> {
    fn orig(&self) -> Option<&TokenStream> {
        self.orig.as_ref()
    }
}

impl<const OPTIONAL: bool> TryFrom<&FXDefault<OPTIONAL>> for String {
    type Error = darling::Error;

    fn try_from(dv: &FXDefault<OPTIONAL>) -> darling::Result<Self> {
        if let Some(NestedMeta::Lit(syn::Lit::Str(ref lit))) = dv.value {
            Ok(lit.value())
        }
        else {
            Err(darling::Error::custom("The default value must be a string")
                .with_span(&dv.orig.as_ref().map_or_else(|| Span::call_site(), |o| o.span())))
        }
    }
}
