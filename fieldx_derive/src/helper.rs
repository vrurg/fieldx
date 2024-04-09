use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, Lit, Meta};

#[derive(Debug)]
pub(crate) enum FXHelperKind {
    Flag(bool),
    Name(String),
}

#[derive(Debug)]
pub(crate) struct FXHelper {
    value: FXHelperKind,
    orig:  Option<Meta>,
}

impl FXHelper {
    pub fn new(value: FXHelperKind, src: Meta) -> Self {
        Self { value, orig: Some(src) }
    }

    pub fn boolish(is_set: bool) -> Self {
        Self {
            value: FXHelperKind::Flag(is_set),
            orig:  None,
        }
    }

    pub fn stringy(name: &str) -> Self {
        Self {
            value: FXHelperKind::Name(String::from(name)),
            orig:  None,
        }
    }

    // pub fn truthy() -> Option<Self> {
    //     Some(FXHelper::boolish(true))
    // }

    pub fn value(&self) -> &FXHelperKind {
        &self.value
    }

    pub fn is_true(&self) -> bool {
        match &self.value {
            FXHelperKind::Flag(b) => *b,
            FXHelperKind::Name(_) => true,
        }
    }
}

impl From<bool> for FXHelper {
    fn from(value: bool) -> Self {
        FXHelper::boolish(value)
    }
}

impl From<&str> for FXHelper {
    fn from(value: &str) -> Self {
        FXHelper::stringy(value)
    }
}

impl ToTokens for FXHelper {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.orig.to_tokens(tokens)
    }
}

impl TryFrom<&Expr> for FXHelperKind {
    type Error = darling::Error;

    fn try_from(expr: &Expr) -> Result<Self, Self::Error> {
        let result: Result<Self, darling::Error> = match expr {
            Expr::Path(ref expr_path) => {
                if expr_path.path.is_ident("on") {
                    Ok(FXHelperKind::Flag(true))
                }
                else if expr_path.path.is_ident("off") {
                    Ok(FXHelperKind::Flag(false))
                }
                else {
                    Err(darling::Error::custom(format!(
                        "Unexpected keyword `{}`",
                        expr_path.to_token_stream()
                    )))
                }
            }
            Expr::Lit(ref expr) => match expr.lit {
                Lit::Bool(ref b) => Ok(FXHelperKind::Flag(b.value)),
                Lit::Str(ref s) => {
                    let helper_name = s.value();
                    if helper_name.len() > 0 {
                        Ok(FXHelperKind::Name(helper_name))
                    }
                    else {
                        Err(darling::Error::custom(format!("Helper method name cannot be empty")))
                    }
                }
                _ => Err(darling::Error::custom(format!(
                    "Unexpected literal `{}`",
                    expr.to_token_stream()
                ))),
            },
            _ => Err(darling::Error::custom("Unexpected value")),
        };

        if let Err(err) = result {
            return Err(
                err
                    .help("Helper options expect either a boolean, or `on`/`off` keywords, or a string with explicit method name")
                    .with_span(expr));
        }

        result
    }
}

impl FromMeta for FXHelper {
    fn from_meta(input: &Meta) -> Result<Self, darling::Error> {
        // eprintln!("HELPER FROM {:#?}", input);
        match input {
            Meta::Path(_orig) => Ok(FXHelper::new(FXHelperKind::Flag(true), input.clone())),
            Meta::NameValue(nv) => {
                // let opt = nv.path.require_ident()?.to_string();
                Ok(FXHelper::new(FXHelperKind::try_from(&nv.value)?, input.clone()))
            }
            Meta::List(_) => Err(darling::Error::custom("Function-like arguments are not supported").with_span(input)),
        }
    }
}
