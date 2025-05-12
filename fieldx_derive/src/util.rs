use fieldx_core::types::meta::FXToksMeta;
use fieldx_core::types::meta::FXValueFlag;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;

#[allow(dead_code)]
// Used by serde generation.
pub(crate) fn derive_toks(traits: &[TokenStream]) -> TokenStream {
    if !traits.is_empty() {
        quote!(#[derive(#( #traits ),*)])
    }
    else {
        quote![]
    }
}

#[cfg(not(debug_assertions))]
#[allow(unused)]
macro_rules! TODO {
    ($message:expr; $dummy:expr) => {
        compile_error!(concat!("TODO: Must implement prior to release: ", $message));
    };
    ($message:expr;) => {
        compile_error!(concat!("TODO: Must implement prior to release: ", $message));
    };
    ($message:expr) => {
        compile_error!(concat!("TODO: Must implement prior to release: ", $message));
    };
}

#[cfg(debug_assertions)]
#[allow(unused)]
macro_rules! dump_tt {
    ($tt:expr) => {{
        dump_tt!("", $tt)
    }};
    ($pfx:expr, $tt:expr) => {{
        let tt = $tt;
        eprintln!("{}{}", $pfx, tt);
        tt
    }};
}

#[cfg(debug_assertions)]
#[allow(unused)]
macro_rules! dump_tt_struct {
    ($tt:expr) => {{
        dump_tt_struct!("", $tt)
    }};
    ($pfx:expr, $tt:expr) => {{
        let tt = $tt;
        eprintln!("{}{:#?}", $pfx, tt);
        tt
    }};
}

#[cfg(not(debug_assertions))]
#[allow(unused)]
macro_rules! dump_tt {
    ($tt:expr) => {
        $tt
    };
}

#[cfg(not(debug_assertions))]
#[allow(unused)]
macro_rules! dump_tt_struct {
    ($tt:expr) => {
        $tt
    };
}

#[cfg(feature = "tracing")]
#[allow(unused_macros)]
macro_rules! fxtrace {
    ( $( $disp:expr ),* ) => {
        eprint!("&&& {}:{}", file!(), line!());
        $( eprint!(" {}", $disp ); )*
        eprintln!();
    };
}

#[cfg(not(feature = "tracing"))]
#[allow(unused_macros)]
macro_rules! fxtrace {
    () => {};
}

#[allow(unused_imports)]
pub(crate) use dump_tt;
#[allow(unused_imports)]
pub(crate) use dump_tt_struct;
#[allow(unused_imports)]
pub(crate) use fxtrace;
use quote::quote_spanned;

#[inline]
pub(crate) fn std_default_expr_toks(span: Span) -> FXToksMeta {
    FXToksMeta::new(
        quote_spanned! {span=> ::std::default::Default::default()},
        FXValueFlag::StdDefault,
    )
}
