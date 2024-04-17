// #![feature(proc_macro_diagnostic)]
mod codegen;
mod fields;
mod helper;
mod input_receiver;
mod util;

// use std::panic::{catch_unwind, set_hook};

use darling::{ast, FromDeriveInput, FromMeta};
use syn::{parse_macro_input, DeriveInput};

use crate::{input_receiver::FXInputReceiver, util::args::FXSArgs};

// use rust_format::{Config, Edition, Formatter, RustFmt};
// #[allow(dead_code)]
// fn prettify_tok(item: TokenStream) -> String {
//     let cfg = Config::new_str().edition(Edition::Rust2021);
//     let rustfmt = RustFmt::from_config(cfg);
//     rustfmt.format_str(item.to_string()).unwrap()
// }

#[proc_macro_attribute]
pub fn fxstruct(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // set_hook(Box::new(|info| {
    //     eprintln!("!!! PANIC HOOK");
    //     if let Some(s) = info.payload().downcast_ref::<&str>() {
    //         eprintln!("panic occurred: {s:?}");
    //     }
    //     if let Some(loc) = info.location() {
    //         eprintln!("!!! PANICED AT {}:{}", loc.file(), loc.line());
    //     }
    // }));

    let attr_args = match ast::NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return darling::Error::from(e).write_errors().into();
        }
    };

    let args = match FXSArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    let input_ast = parse_macro_input!(input as DeriveInput);
    let fx = match FXInputReceiver::from_derive_input(&input_ast) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    codegen::FXRewriter::new(fx, args).rewrite().into()
}
