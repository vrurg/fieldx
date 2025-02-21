pub mod codegen_trait;
mod method_constructor;
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

#[derive(PartialEq, Clone, Debug)]
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

    // #[cfg(feature = "serde")]
    // pub(crate) fn unwrap_or(self, default: T) -> T {
    //     match self {
    //         FXValueRepr::None => default,
    //         FXValueRepr::Exact(v) => v,
    //         FXValueRepr::Versatile(v) => v,
    //     }
    // }

    #[cfg(feature = "serde")]
    pub(crate) fn unwrap_or_else(self, default_fn: impl FnOnce() -> T) -> T {
        match self {
            FXValueRepr::None => default_fn(),
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

pub struct FXRewriter<'a> {
    codegen_ctx: Rc<FXCodeGenCtx>,
    plain:       OnceCell<FXCodeGenerator<'a>>,
    sync:        OnceCell<FXCodeGenerator<'a>>,
}

impl<'a> FXRewriter<'a> {
    pub fn new(input: FXInputReceiver, args: FXSArgs) -> Self {
        let ctx = Rc::new(FXCodeGenCtx::new(input, args));

        Self {
            codegen_ctx: ctx,
            plain:       OnceCell::new(),
            sync:        OnceCell::new(),
        }
    }

    pub fn plain_gen(&'a self) -> &'a FXCodeGenerator<'a> {
        self.plain
            .get_or_init(|| FXCodeGenerator::ModePlain(FXCodeGenPlain::new(self, self.codegen_ctx.clone())))
    }

    pub fn sync_gen(&'a self) -> &'a FXCodeGenerator<'a> {
        self.sync
            .get_or_init(|| FXCodeGenerator::ModeSync(FXCodeGenSync::new(self, self.codegen_ctx.clone())))
    }

    pub fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.codegen_ctx
    }

    pub fn field_codegen(&'a self, fctx: &FXFieldCtx) -> darling::Result<&'a FXCodeGenerator<'a>> {
        Ok(if fctx.is_plain() {
            self.plain_gen()
        }
        else {
            // Sync or async go here
            self.sync_gen()
        })
    }

    pub fn struct_codegen(&'a self) -> &'a FXCodeGenerator<'a> {
        if self.ctx().is_syncish() {
            self.sync_gen()
        }
        else {
            self.plain_gen()
        }
    }

    pub fn rewrite(&'a mut self) -> TokenStream {
        self.prepare_struct();
        self.rewrite_struct();
        self.finalize()
    }

    fn prepare_ref_counted(&'a self) {
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_ref_counted() {
            #[allow(unused_mut)]
            let mut fieldx_args: Vec<TokenStream> = vec![quote![skip]];
            #[cfg(feature = "serde")]
            fieldx_args.push(quote![serde(off)]);

            // Safe because of is_ref_counted
            let myself_field = ctx.myself_field().unwrap();
            let (_, weak_type) = if ctx.is_syncish() {
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

    fn prepare_struct(&'a self) {
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

    fn prepare_field(&'a self, fctx: Ref<FXFieldCtx>) -> darling::Result<()> {
        let ctx = self.ctx();
        let is_active = !fctx.is_skipped();

        if is_active {
            if fctx.needs_accessor() && fctx.is_copy() {
                ctx.add_for_copy_trait_check(&fctx);
            }
        }

        let codegen = self.field_codegen(&fctx)?;
        codegen.field_default(&fctx)?;
        if is_active {
            codegen.field_methods(&fctx)?;
        }

        // Has to always be the last here as it may use attributes added by the previous methods.
        codegen.field_decl(&fctx)?;

        Ok(())
    }

    fn rewrite_struct(&'a self) {
        let ctx = self.ctx();

        self.struct_extras();

        if ctx.needs_builder_struct() {
            let builder_ident = ctx.builder_ident();
            let span = ctx.helper_span(FXHelperKind::Builder);
            let generic_params = ctx.struct_generic_params();
            let vis = self.ctx().input().vis();
            ctx.add_method_decl(quote_spanned! {span=>
                #[inline]
                #vis fn builder() -> #builder_ident #generic_params {
                    #builder_ident::new()
                }
            })
        }

        #[cfg(feature = "serde")]
        self.serde_rewrite_struct();
    }

    fn struct_extras(&'a self) {
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
            let body = cgen.maybe_ref_counted_create(&quote![Self], &quote![..Self::default()], None);

            ctx.add_method_decl(quote![
                #[inline]
                pub fn #new_name() -> #return_type {
                    #body
                }
            ]);
        }
    }

    fn myself_methods(&'a self) {
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
        let (impl_generics, type_generics, where_clause) = ctx.input().generics().split_for_impl();

        if !defaults.is_empty() {
            quote! [
                impl #impl_generics Default for #ident #type_generics #where_clause {
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

    fn builder_field_ctxs(&self) -> Vec<darling::Result<Ref<FXFieldCtx>>> {
        let ctx = self.ctx();
        let builder_field_idents = ctx.builder_field_ident().borrow();
        builder_field_idents
            .iter()
            .map(|ident| ctx.ident_field_ctx(&ident))
            .collect()
    }

    fn builder_impl(&'a self) -> TokenStream {
        let ctx = self.ctx();
        let span = ctx.helper_span(FXHelperKind::Builder);
        let vis = ctx.builder_struct_visibility();
        let builder_ident = ctx.builder_ident();
        let builders = ctx.builders_combined();
        let input_ident = ctx.input_ident();
        let (impl_generics, _, where_clause) = ctx.input().generics().split_for_impl();
        let generic_params = ctx.struct_generic_params();
        let attributes = ctx.args().builder_impl_attributes();
        let post_build_ident = ctx.builder_post_build_ident();

        let mut field_setters = Vec::<TokenStream>::new();
        let mut use_default = false;
        let mut builder_checkers = vec![];
        let mut fields_new = vec![];
        if let Some(myself_field) = ctx.myself_field() {
            fields_new.push(quote_spanned! {span=> #myself_field: ::std::default::Default::default() });
        }
        for fctx in self.builder_field_ctxs() {
            if let Ok(fctx) = fctx {
                let ident = fctx.ident();
                fields_new.push(quote_spanned! {span=> #ident: None });

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
                ctx.push_error(fctx.unwrap_err());
            }
        }

        let default_initializer = if use_default && ctx.needs_default() {
            quote_spanned! {span=> ..::std::default::Default::default()}
        }
        else {
            quote![]
        };

        let cgen = self.struct_codegen();
        let builder_return_type = cgen.maybe_ref_counted(&cgen.builder_return_type());

        let fn_new = quote_spanned! {span=>
            #vis fn new() -> Self {
                Self {
                    #( #fields_new ),*
                }
            }
        };

        let construction = cgen.maybe_ref_counted_create(
            &input_ident.to_token_stream(),
            &quote_spanned! {span=>
                    #(#field_setters,)*
                    #default_initializer
            },
            post_build_ident,
        );

        let builder_error_type = if let Some(error_type) = ctx.builder_error_type() {
            quote_spanned![span=> #error_type]
        }
        else {
            quote_spanned! {span=> ::fieldx::error::FieldXError}
        };

        quote_spanned! {span=>
            #attributes
            impl #impl_generics #builder_ident #generic_params
            #where_clause
            {
                #fn_new
                #builders
                #vis fn build(&mut self) -> ::std::result::Result<#builder_return_type, #builder_error_type> {
                    #( #builder_checkers );*
                    Ok(#construction)
                }
            }
        }
    }

    fn builder_struct(&'a self) -> TokenStream {
        let ctx = self.ctx();

        if ctx.needs_builder_struct() {
            let args = ctx.args();
            let cgen = self.struct_codegen();
            let builder_fields = ctx.builder_fields_combined();
            let builder_impl = self.builder_impl();
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;
            let span = ctx.helper_span(FXHelperKind::Builder);
            let vis = ctx.builder_struct_visibility();
            let attributes = args.builder_attributes();
            let builder_ident = ctx.builder_ident();

            let myself_field = if args.is_ref_counted() {
                let (_, weak_type) = cgen.ref_count_types();
                let mf = ctx.myself_field();
                let input_ident = ctx.input_ident();
                quote_spanned![span=> #mf: #weak_type<#input_ident #generics>,]
            }
            else {
                quote![]
            };

            quote_spanned! {span=>
                // #derive_attr
                #attributes
                #vis struct #builder_ident #generics
                #where_clause
                {
                    #myself_field
                    #builder_fields
                }

                #builder_impl
            }
        }
        else {
            quote![]
        }
    }

    fn finalize(&'a self) -> TokenStream {
        let ctx = self.ctx();

        let &FXInputReceiver {
            ref vis,
            ref ident,
            ref generics,
            ..
        } = ctx.input();

        let span = ctx.input().ident().span();

        // ctx.add_attr(self.derive_toks(&self.derive_traits()));

        let attributes = ctx.all_attrs();
        let attributes_impl = ctx.args().attributes_impl().as_ref();
        let methods = ctx.methods_combined();
        let fields = ctx.struct_fields();
        let default = self.default_impl();
        let builder_struct = self.builder_struct();
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

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

        ctx.tokens_extend(quote_spanned! [span=>
            #[allow(unused_imports)]
            use ::fieldx::traits::*;

            #( #attributes )*
            #vis struct #ident #generics
            #where_clause
            {
                #( #fields ),*
            }

            impl #impl_generics FXStruct for #ident #type_generics #where_clause {}

            #attributes_impl
            impl #impl_generics #ident #type_generics #where_clause {
                #methods
                #copyable_validation
            }

            #default

            #builder_struct
        ]);
        ctx.finalize()
    }
}
