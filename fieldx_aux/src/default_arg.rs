//! Default value.

use crate::{FXOrig, FXTriggerHelper, FromNestAttr};
use darling::FromMeta;
use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{parse2, spanned::Spanned, ExprCall, Meta};

/// Default value argument.
///
/// Normally, looks like `default(42)` or `default(Type::func())`, or just `default`.
#[derive(Debug, Clone, Getters)]
pub struct FXDefault {
    off:   bool,
    /// The default value literal or path.
    value: Option<syn::Expr>,
    /// The original tokens used to produce this object.
    orig:  Option<TokenStream>,
}

impl FXDefault {
    /// True if a value is explicitly specified.
    #[allow(dead_code)]
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub fn off(&self) -> bool {
        self.off
    }

    pub fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref()
    }

    pub fn is_str(&self) -> bool {
        if let Some(syn::Expr::Lit(lit)) = &self.value {
            if let syn::Lit::Str(_) = lit.lit {
                return true;
            }
        }
        false
    }

    fn from_call_like(call: ExprCall) -> darling::Result<Self> {
        let arg_count = call.args.len();
        if arg_count == 0 {
            return Err(darling::Error::too_few_items(1));
        }
        if arg_count > 2 {
            return Err(darling::Error::too_many_items(2));
        }

        match arg_count {
            1 => {
                let value = Some(call.args[0].clone());
                Ok(Self {
                    off: false,
                    value,
                    orig: Some(call.to_token_stream()),
                })
            }
            2 => {
                let off = if let syn::Expr::Path(path) = &call.args[0] {
                    path.path.is_ident("off")
                }
                else {
                    return Err(darling::Error::custom("The first argument must be 'off' keyword")
                        .with_span(&call.args[0].span()));
                };

                if off {
                    let value = Some(call.args[1].clone());
                    Ok(Self {
                        off,
                        value,
                        orig: Some(call.to_token_stream()),
                    })
                }
                else {
                    Err(darling::Error::custom("The first argument must be 'off' keyword")
                        .with_span(&call.args[0].span()))
                }
            }
            _ => unreachable!(),
        }
    }
}

impl FromMeta for FXDefault {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        match item {
            Meta::Path(path) => Ok(Self {
                off:   false,
                value: None,
                orig:  Some(path.to_token_stream()),
            }),
            Meta::List(_list) => {
                let syn::Expr::Call(expr) = parse2(item.to_token_stream())?
                else {
                    // It should be impossible for Meta::List to not have a call-like syntax.
                    return Err(darling::Error::custom("Expected call-like syntax default(...)"));
                };
                Self::from_call_like(expr)
            }
            Meta::NameValue(name_value) => Ok(Self {
                off:   false,
                value: Some(name_value.value.clone()),
                orig:  Some(name_value.to_token_stream()),
            }),
        }
    }
}

impl FXTriggerHelper for FXDefault {
    fn is_true(&self) -> bool {
        !self.off
    }
}

impl FXOrig<TokenStream> for FXDefault {
    fn orig(&self) -> Option<&TokenStream> {
        self.orig.as_ref()
    }
}

impl FromNestAttr for FXDefault {
    fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
        Ok(Self {
            off:   false,
            value: None,
            orig:  Some(path.to_token_stream()),
        })
    }
}

impl TryFrom<&FXDefault> for String {
    type Error = darling::Error;

    fn try_from(dv: &FXDefault) -> darling::Result<Self> {
        if let Some(syn::Expr::Lit(lit)) = &dv.value {
            if let syn::Lit::Str(str) = &lit.lit {
                return Ok(str.value());
            }
        }
        Err(darling::Error::custom("The default value must be a string")
            .with_span(&dv.orig.as_ref().map_or_else(|| Span::call_site(), |o| o.span())))
    }
}
