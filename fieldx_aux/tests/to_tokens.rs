use darling::FromMeta;
use fieldx_aux::*;
use quote::quote;
use quote::ToTokens;

#[test]
fn test_to_tokens() {
    // String
    let input = quote! {
        test(off, "ok")
    };
    let meta: syn::Meta = syn::parse2(input.clone()).unwrap();
    let parsed = FXValue::<String>::from_meta(&meta).unwrap();
    assert_eq!(input.to_string(), parsed.to_token_stream().to_string());

    // bool/keyword
    let input = quote! {
        test
    };
    let meta: syn::Meta = syn::parse2(input.clone()).unwrap();
    let parsed = FXBool::from_meta(&meta).unwrap();
    // Keywords do not round-trip to the original input because FXNestingAttr is agnostic about its inner type syntax
    // and always wraps it in parentheses.
    assert_eq!(quote! {test()}.to_string(), parsed.to_token_stream().to_string());

    // float
    let input = quote! {
        test(3.1415926)
    };
    let meta: syn::Meta = syn::parse2(input.clone()).unwrap();
    let parsed = FXValue::<f32>::from_meta(&meta).unwrap();
    assert_eq!(input.to_string(), parsed.to_token_stream().to_string());
}
