pub mod codegen_trait;
mod plain;
#[cfg(feature = "serde")]
mod serde;
mod sync;

use crate::{
    ctx::{codegen::FXCodeGenCtx, field::FXFieldCtx},
    fields::FXField,
    helper::*,
    util::args::FXSArgs,
    FXInputReceiver,
};
pub use codegen_trait::FXCodeGenContextual;
use codegen_trait::FXCodeGenerator;
use darling::FromField;
pub use plain::FXCodeGenPlain;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
#[cfg(feature = "serde")]
use serde::FXRewriteSerde;
use std::{
    cell::{OnceCell, Ref},
    rc::Rc,
};
use syn::{parse_quote, spanned::Spanned};
pub use sync::FXCodeGenSync;

#[allow(dead_code)]
pub enum FXInlining {
    Default,
    Inline,
    Always,
}

#[derive(PartialEq)]
pub(crate) enum FXValueRepr<T> {
    None,
    Exact(T),
    Versatile(T),
}

impl<T> FXValueRepr<T> {
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, FXValueRepr::None)
    }

    pub(crate) fn expect(self, msg: &str) -> T {
        match self {
            FXValueRepr::None => panic!("{}", msg),
            FXValueRepr::Exact(v) => v,
            FXValueRepr::Versatile(v) => v,
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn unwrap_or(self, default: T) -> T {
        match self {
            FXValueRepr::None => default,
            FXValueRepr::Exact(v) => v,
            FXValueRepr::Versatile(v) => v,
        }
    }

    pub(crate) fn map<U>(self, mapper: impl FnOnce(T) -> U) -> FXValueRepr<U> {
        match self {
            FXValueRepr::None => FXValueRepr::None,
            FXValueRepr::Exact(v) => FXValueRepr::Exact(mapper(v)),
            FXValueRepr::Versatile(v) => FXValueRepr::Versatile(mapper(v)),
        }
    }
}

// Methods that are related to the current context if first place.

pub struct FXRewriter {
    codegen_ctx: Rc<FXCodeGenCtx>,
    plain:       OnceCell<FXCodeGenerator>,
    sync:        OnceCell<FXCodeGenerator>,
}

impl FXRewriter {
    pub fn new(input: FXInputReceiver, args: FXSArgs) -> Self {
        let ctx = Rc::new(FXCodeGenCtx::new(input, args));

        Self {
            codegen_ctx: ctx,
            plain:       OnceCell::new(),
            sync:        OnceCell::new(),
        }
    }

    pub fn plain_gen(&self) -> &FXCodeGenerator {
        self.plain
            .get_or_init(|| FXCodeGenerator::ModePlain(FXCodeGenPlain::new(self.codegen_ctx.clone())))
    }

    pub fn sync_gen(&self) -> &FXCodeGenerator {
        self.sync
            .get_or_init(|| FXCodeGenerator::ModeSync(FXCodeGenSync::new(self.codegen_ctx.clone())))
    }

    pub fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.codegen_ctx
    }

    pub fn field_codegen(&self, fctx: &FXFieldCtx) -> darling::Result<&FXCodeGenerator> {
        Ok(if fctx.is_sync() {
            self.sync_gen()
        }
        else {
            self.plain_gen()
        })
    }

    pub fn struct_codegen(&self) -> &FXCodeGenerator {
        if self.ctx().is_rather_sync() {
            self.sync_gen()
        }
        else {
            self.plain_gen()
        }
    }

    pub fn rewrite(&mut self) -> TokenStream {
        self.prepare_struct();
        self.rewrite_struct();
        self.finalize()
    }

    fn prepare_ref_counted(&self) {
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_ref_counted() {
            #[allow(unused_mut)]
            let mut fieldx_args: Vec<TokenStream> = vec![quote![skip], quote![builder(off)]];
            #[cfg(feature = "serde")]
            fieldx_args.push(quote![serde(off)]);

            // Safe because of is_ref_counted
            let myself_field = ctx.myself_field().unwrap();
            let (_, weak_type) = if ctx.is_rather_sync() {
                self.sync_gen().ref_count_types()
            }
            else {
                self.plain_gen().ref_count_types()
            };

            let field: syn::Field = parse_quote![
                #[fieldx( #( #fieldx_args ),* )]
                #myself_field: #weak_type<Self>
            ];

            ctx.exec_or_record(|| {
                let field = FXField::from_field(&field)?;
                self.ctx().add_field(field);
                Ok(())
            });
        }
    }

    fn prepare_struct(&self) {
        self.prepare_ref_counted();
        let ctx = self.ctx();

        for field in self.ctx().all_fields() {
            let Ok(fctx) = ctx.field_ctx(field)
            else {
                continue;
            };
            ctx.ok_or_record(self.prepare_field(fctx));
        }

        #[cfg(feature = "serde")]
        self.serde_prepare_struct();
    }

    fn prepare_field(&self, fctx: Ref<FXFieldCtx>) -> darling::Result<()> {
        let ctx = self.ctx();

        if fctx.needs_accessor() && fctx.is_copy() {
            ctx.add_for_copy_trait_check(&fctx);
        }

        let codegen = self.field_codegen(&fctx)?;

        // eprintln!(
        //     "USING {} codegenerator for field {} of {}",
        //     match codegen {
        //         FXCodeGenerator::ModePlain(_) => "plain",
        //         FXCodeGenerator::ModeSync(_) => "sync",
        //     },
        //     fctx.ident(),
        //     ctx.input_ident()
        // );
        // let field = fctx.field();
        // eprintln!(
        //     "MODES: field is sync? {:?}; struct is sync? {:?}; fctx final is {}\n  mode_sync: {:?}\n  mode_async: {:?}",
        //     fctx.field().is_sync(),
        //     ctx.is_rather_sync(),
        //     fctx.is_sync(),
        //     field.mode_sync(),
        //     field.mode_async(),
        // );

        codegen.field_default(&fctx)?;
        codegen.field_methods(&fctx)?;

        // Has to always be the last here as it may use attributes added by the previous methods.
        codegen.field_decl(&fctx);

        Ok(())
    }

    fn rewrite_struct(&self) {
        let ctx = self.ctx();

        self.struct_extras();

        if ctx.needs_builder_struct() {
            let builder_ident = ctx.builder_ident();
            let generic_params = ctx.struct_generic_params();
            let vis = self.ctx().input().vis();
            ctx.add_method_decl(quote![
                #[inline]
                #vis fn builder() -> #builder_ident #generic_params {
                    #builder_ident::default()
                }
            ])
        }

        #[cfg(feature = "serde")]
        self.serde_rewrite_struct();
    }

    fn struct_extras(&self) {
        let ctx = self.ctx();
        let cgen = self.struct_codegen();

        self.myself_methods();

        if ctx.needs_default() {
            // Generate fn new()
            let new_name = if ctx.args().needs_new() {
                quote![new]
            }
            else {
                quote![__fieldx_new]
            };

            let return_type = cgen.maybe_ref_counted(&quote![Self]);
            let body = cgen.maybe_ref_counted_create(&quote![Self], &quote![..Self::default()]);

            ctx.add_method_decl(quote![
                #[inline]
                pub fn #new_name() -> #return_type {
                    #body
                }
            ]);
        }
    }

    fn myself_methods(&self) {
        let ctx = self.ctx();
        let args = ctx.args();

        if args.is_ref_counted() {
            let rc_helper = args.rc().as_ref().unwrap();
            let rc_span = rc_helper.orig().map_or_else(|| Span::call_site(), |orig| orig.span());
            let (myself_name, downgrade_name) = ctx.myself_names().unwrap();
            let myself_field = ctx.myself_field();
            let (rc_type, weak_type) = self.struct_codegen().ref_count_types();
            let visibility = rc_helper.public_mode();

            ctx.add_method_decl(quote_spanned![rc_span=>
                #[allow(dead_code)]
                #[inline(always)]
                #visibility fn #myself_name(&self) -> ::std::option::Option<#rc_type<Self>> {
                    #weak_type::upgrade(&self.#myself_field)
                }
            ]);
            ctx.add_method_decl(quote_spanned![rc_span=>
                #[allow(dead_code)]
                #[inline(always)]
                #visibility fn #downgrade_name(&self) -> #weak_type<Self> {
                    #weak_type::clone(&self.#myself_field)
                }
            ]);
        }
    }

    fn default_impl(&self) -> TokenStream {
        let ctx = self.ctx();

        if !ctx.needs_default() {
            return quote![];
        }

        let defaults = ctx.defaults_combined();
        let ident = ctx.input().ident();
        let generics = ctx.input().generics();
        let where_clause = &generics.where_clause;

        if !defaults.is_empty() {
            quote! [
                impl #generics Default for #ident #generics #where_clause {
                    fn default() -> Self {
                        Self { #defaults }
                    }
                }
            ]
        }
        else {
            // It's already empty, what sense in allocating another copy?
            defaults
        }
    }

    fn builder_struct_visibility(&self) -> TokenStream {
        let ctx = self.ctx();
        self.ctx()
            .args()
            .get_helper(FXHelperKind::Builder)
            .and_then(|builder| builder.public_mode().map(|pm| pm.to_token_stream()))
            .or_else(|| Some(ctx.input().vis().to_token_stream()))
            .unwrap()
    }

    fn builder_field_ctxs(&self) -> Vec<darling::Result<Ref<FXFieldCtx>>> {
        let ctx = self.ctx();
        let builder_field_idents = ctx.builder_field_ident().borrow();
        builder_field_idents
            .iter()
            .map(|ident| ctx.ident_field_ctx(&ident))
            .collect()
    }

    fn builder_impl(&self) -> TokenStream {
        let ctx = self.ctx();
        let vis = self.builder_struct_visibility();
        let builder_ident = ctx.builder_ident();
        let builders = ctx.builders_combined();
        let input_ident = ctx.input_ident();
        let generics = ctx.input().generics();
        let where_clause = &generics.where_clause;
        let generic_params = ctx.struct_generic_params();
        let attributes = ctx.args().builder_impl_attributes();

        let mut field_setters = Vec::<TokenStream>::new();
        let mut use_default = false;
        let mut builder_checkers = vec![];
        for fctx in self.builder_field_ctxs() {
            if let Ok(fctx) = fctx {
                let fgen = match self.field_codegen(&fctx) {
                    Ok(fgen) => fgen,
                    Err(err) => {
                        ctx.push_error(err);
                        continue;
                    }
                };
                fgen.field_builder_value_required(&fctx);
                let fsetter = ctx.ok_or_empty(fgen.field_builder_setter(&fctx));
                if fsetter.is_empty() {
                    use_default = true;
                }
                else {
                    field_setters.push(fsetter);
                }
                if let Some(bchecker) = fctx.builder_checker() {
                    builder_checkers.push(bchecker);
                }
            }
            else {
                self.ctx().push_error(fctx.unwrap_err());
            }
        }

        let default_initializer = if use_default {
            quote![..::std::default::Default::default()]
        }
        else {
            quote![]
        };

        let cgen = self.struct_codegen();
        let builder_return_type = cgen.maybe_ref_counted(&cgen.builder_return_type());

        let construction = cgen.maybe_ref_counted_create(
            &input_ident.to_token_stream(),
            &quote![
                    #(#field_setters,)*
                    #default_initializer
            ],
        );

        quote![
            #attributes
            impl #generics #builder_ident #generic_params
            #where_clause
            {
                #builders
                #vis fn build(&mut self) -> ::std::result::Result<#builder_return_type, ::fieldx::errors::FieldXError> {
                    #( #builder_checkers );*
                    Ok(#construction)
                }
            }
        ]
    }

    fn builder_struct(&self) -> TokenStream {
        let ctx = self.ctx();

        if ctx.needs_builder_struct() {
            let args = ctx.args();
            let cgen = self.struct_codegen();
            let builder_fields = ctx.builder_fields_combined();
            let builder_impl = self.builder_impl();
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;
            let span = ctx.helper_span(FXHelperKind::Builder);
            let vis = self.builder_struct_visibility();
            let attributes = args.builder_attributes();
            let traits = vec![quote![Default]];
            let derive_attr = crate::util::derive_toks(&traits);
            let builder_ident = ctx.builder_ident();

            let myself_field = if args.is_ref_counted() {
                let (_, weak_type) = cgen.ref_count_types();
                let mf = ctx.myself_field();
                let input_ident = ctx.input_ident();
                quote![#mf: #weak_type<#input_ident #generics>,]
            }
            else {
                quote![]
            };

            quote_spanned![span=>
                #derive_attr
                #attributes
                #vis struct #builder_ident #generics
                #where_clause
                {
                    #myself_field
                    #builder_fields
                }

                #builder_impl
            ]
        }
        else {
            quote![]
        }
    }

    fn finalize(&self) -> TokenStream {
        let ctx = self.ctx();

        let &FXInputReceiver {
            ref vis,
            ref ident,
            ref generics,
            ..
        } = ctx.input();

        // ctx.add_attr(self.derive_toks(&self.derive_traits()));

        let attributes = ctx.all_attrs();
        let attributes_impl = ctx.args().attributes_impl().as_ref();
        let methods = ctx.methods_combined();
        let fields = ctx.struct_fields();
        let default = self.default_impl();
        let builder_struct = self.builder_struct();
        let where_clause = &generics.where_clause;
        let generic_params = ctx.struct_generic_params();

        let copyables = ctx.copyable_types();
        let copyable_validation = if !copyables.is_empty() {
            let copyables: Vec<TokenStream> = copyables.iter().map(|ct| ct.to_token_stream()).collect();
            Some(quote![
                #[allow(dead_code)]
                fn __fieldx_copy_validation() {
                    fn field_implements_copy<T: ?Sized + Copy>() {}
                    #( field_implements_copy::<#copyables>(); )*
                }
            ])
        }
        else {
            None
        };

        ctx.tokens_extend(quote! [
            use ::fieldx::traits::*;

            #( #attributes )*
            #vis struct #ident #generics
            #where_clause
            {
                #( #fields ),*
            }

            impl #generics FXStruct for #ident #generic_params #where_clause {}

            #attributes_impl
            impl #generics #ident #generics #where_clause {
                #methods
                #copyable_validation
            }

            #default

            #builder_struct
        ]);
        ctx.finalize()
    }
}
