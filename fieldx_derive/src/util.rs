pub(crate) mod args;
use proc_macro2::TokenStream;
use quote::quote;

#[allow(dead_code)]
// Used by serde generation.
pub fn derive_toks(traits: &[TokenStream]) -> TokenStream {
    if traits.len() > 0 {
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
macro_rules! TODO {
    ($message:expr; $dummy:expr) => {{
        $dummy
    }};
    ($message:expr;) => {{
        unimplemented!($message)
    }};
    ($message:expr) => {{
        unimplemented!($message)
    }};
}

macro_rules! needs_helper {
    ( $( $field:ident ),+ ) => {
        ::paste::paste!{
            $(
                #[inline]
                pub fn [<needs_ $field>](&self) -> Option<bool> {
                    use crate::helper::FXTriggerHelper;
                    self.$field.as_ref().map(|h| h.is_true())
                }
            )+
        }
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
pub(crate) use fxtrace;
pub(crate) use needs_helper;
#[allow(unused_imports)]
pub(crate) use TODO;
// pub(crate) use self::helper_std_fields;
