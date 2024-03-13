// #![feature(proc_macro_diagnostic)]
mod fields;
mod helper;
mod input_receiver;
mod util;
mod codegen;

use darling::{ast, FromDeriveInput, FromMeta};
use proc_macro2::TokenStream;
use rust_format::{Config, Edition, Formatter, RustFmt};
use syn::{parse_macro_input, DeriveInput};

use crate::{input_receiver::FXInputReceiver, util::args::FXSArgs};

// #[proc_macro_derive(FieldX, attributes(fieldx))]
// pub fn fieldx_derive(input: TokenStream) -> TokenStream {
//     eprintln!("+++ FIELDX DERIVE");

//     // let ast = parse_macro_input!(input as DeriveInput);
//     // let fx = FXInputReceiver::from_derive_input(&ast);

//     // fx.unwrap().rewrite()

//     eprintln!("<<< FIELDX DERIVE");
//     (quote! {}).into()
// }

#[allow(dead_code)]
fn prettify_tok(item: TokenStream) -> String {
    let cfg = Config::new_str().edition(Edition::Rust2021);
    let rustfmt = RustFmt::from_config(cfg);
    rustfmt.format_str(item.to_string()).unwrap()
}

#[proc_macro_attribute]
pub fn fxstruct(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // eprintln!("--- FIELDX ATTRIBUTE");
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

    // eprintln!("&&& ARGS {:#?}", args);
    // eprintln!("<<< INPUT:\n{}", input.to_string());
    // eprintln!("<<< Input span: {:?}", input.span());

    let input_ast = parse_macro_input!(input as DeriveInput);
    let fx = match FXInputReceiver::from_derive_input(&input_ast) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    codegen::FXRewriter::new(fx, &args).rewrite().into()
}
