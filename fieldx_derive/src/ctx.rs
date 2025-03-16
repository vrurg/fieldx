pub(crate) mod codegen;
pub(crate) mod field;
pub(crate) use codegen::FXCodeGenCtx;
pub(crate) use field::FXFieldCtx;

use quote::ToTokens;

pub(crate) struct Attributizer(Vec<syn::Attribute>);

impl Attributizer {
    pub(crate) fn into_inner(self) -> Vec<syn::Attribute> {
        self.0
    }

    pub(crate) fn parse<T: ToTokens>(attrs: T) -> syn::Result<Self> {
        syn::parse2(attrs.to_token_stream())
    }
}

impl syn::parse::Parse for Attributizer {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        Ok(Self(attrs))
    }
}

#[cfg(test)]
mod test {
    use super::Attributizer;

    #[test]
    fn test_attributizer() {
        let attrs = Attributizer::parse(quote::quote! {
            #[derive(Debug)]
            #[cfg(test)]
            #[allow(dead_code)]
        })
        .expect("Parse attributes:");

        let inner_attrs = attrs.into_inner();
        assert_eq!(inner_attrs.len(), 3);
        assert!(inner_attrs[0].path().is_ident("derive"));
        assert!(inner_attrs[1].path().is_ident("cfg"));
        assert!(inner_attrs[2].path().is_ident("allow"));
    }
}
