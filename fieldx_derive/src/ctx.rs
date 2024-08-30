pub mod codegen;
pub mod field;
pub use codegen::FXCodeGenCtx;
pub use field::FXFieldCtx;
use quote::ToTokens;

struct Attributizer(Option<syn::Attribute>);

impl Attributizer {
    fn into_inner(self) -> Option<syn::Attribute> {
        self.0
    }
}

impl<T> From<T> for Attributizer
where
    T: ToTokens,
{
    fn from(attr: T) -> Self {
        let toks = attr.to_token_stream();
        if toks.is_empty() {
            Self(None)
        }
        else {
            Self(Some(syn::parse_quote!(#attr)))
        }
    }
}
