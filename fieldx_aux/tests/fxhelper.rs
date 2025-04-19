use darling::util::Flag;
use darling::*;
use fieldx_aux::*;
use fieldx_derive_support::fxhelper;
use getset::Getters;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;

#[fxhelper(to_tokens)]
#[derive(Debug, Default)]
struct TestHelper {
    #[getset(get = "pub")]
    val: Option<FXString>,
}

impl FromNestAttr for TestHelper {
    set_literals! {helper, ..1usize => val}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

#[test]
fn test_helper() {
    // Make sure that helper to tokens roundtrips correctly.
    let helper = FXNestingAttr::<TestHelper>::from_tokens(quote! { t::test("ok")}).unwrap();
    let helper_toks = helper.to_token_stream();
    let helper2 = FXNestingAttr::<TestHelper>::from_tokens(helper_toks).unwrap();
    assert_eq!(
        helper.to_token_stream().to_string(),
        helper2.to_token_stream().to_string(),
        "helper tokens should be equal"
    );
}
