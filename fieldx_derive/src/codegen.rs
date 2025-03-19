pub(crate) mod codegen_trait;
pub(crate) mod constructor;
mod plain;
#[cfg(feature = "serde")]
mod serde;
pub(crate) mod sync;

use crate::{
    ctx::{codegen::FXCodeGenCtx, field::FXFieldCtx},
    field_receiver::FXField,
    helper::*,
    util::args::FXSArgs,
    FXInputReceiver,
};
pub(crate) use codegen_trait::FXCodeGenContextual;
use codegen_trait::FXCodeGenerator;
use constructor::{FXConstructor, FXFnConstructor, FXImplConstructor};
use darling::FromField;
use fieldx_aux::FXProp;
pub(crate) use plain::FXCodeGenPlain;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
#[cfg(feature = "serde")]
use serde::FXRewriteSerde;
use std::{cell::OnceCell, rc::Rc};
use syn::{parse_quote_spanned, spanned::Spanned};
pub(crate) use sync::FXCodeGenSync;

#[allow(dead_code)]
pub(crate) enum FXInlining {
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

pub(crate) struct FXRewriter<'a> {
    codegen_ctx: Rc<FXCodeGenCtx>,
    plain:       OnceCell<FXCodeGenerator<'a>>,
    sync:        OnceCell<FXCodeGenerator<'a>>,
}

impl<'a> FXRewriter<'a> {
    pub(crate) fn new(input: FXInputReceiver, args: FXSArgs) -> Self {
        let ctx = FXCodeGenCtx::new(input, args);

        Self {
            codegen_ctx: ctx,
            plain:       OnceCell::new(),
            sync:        OnceCell::new(),
        }
    }

    pub(crate) fn plain_gen(&'a self) -> &'a FXCodeGenerator<'a> {
        self.plain
            .get_or_init(|| FXCodeGenerator::ModePlain(FXCodeGenPlain::new(self, self.codegen_ctx.clone())))
    }

    pub(crate) fn sync_gen(&'a self) -> &'a FXCodeGenerator<'a> {
        self.sync
            .get_or_init(|| FXCodeGenerator::ModeSync(FXCodeGenSync::new(self, self.codegen_ctx.clone())))
    }

    pub(crate) fn ctx(&self) -> &Rc<FXCodeGenCtx> {
        &self.codegen_ctx
    }

    pub(crate) fn field_codegen(&'a self, fctx: &FXFieldCtx) -> darling::Result<&'a FXCodeGenerator<'a>> {
        Ok(if *fctx.mode_plain() {
            self.plain_gen()
        }
        else {
            // Sync or async go here
            self.sync_gen()
        })
    }

    pub(crate) fn struct_codegen(&'a self) -> &'a FXCodeGenerator<'a> {
        if *self.ctx().syncish() {
            self.sync_gen()
        }
        else {
            self.plain_gen()
        }
    }

    pub(crate) fn rewrite(&'a mut self) -> TokenStream {
        self.prepare_struct();
        self.rewrite_struct();
        self.finalize()
    }

    fn prepare_ref_counted(&'a self) {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let rc = arg_props.rc();

        if *rc {
            let rc_span = rc.final_span();
            #[allow(unused_mut)]
            let mut fieldx_args: Vec<TokenStream> = vec![quote_spanned![rc_span=> skip]];
            #[cfg(feature = "serde")]
            fieldx_args.push(quote_spanned![rc_span=> serde(off)]);

            // Safe because of is_ref_counted
            let myself_field = arg_props.myself_field_ident().unwrap();
            let (_, weak_type) = if *ctx.syncish() {
                self.sync_gen().ref_count_types(rc_span)
            }
            else {
                self.plain_gen().ref_count_types(rc_span)
            };

            let field: syn::Field = parse_quote_spanned![rc.final_span()=>
                #[fieldx( #( #fieldx_args ),* )]
                #myself_field: #weak_type<Self>
            ];

            ctx.exec_or_record(|| {
                let field = FXField::from_field(&field)?;
                self.ctx().add_extra_field(field);
                Ok(())
            });
        }
    }

    fn prepare_struct(&'a self) {
        self.prepare_ref_counted();
        let ctx = self.ctx();

        for fctx in self.ctx().all_field_ctx() {
            ctx.ok_or_record(self.prepare_field(&fctx));
        }

        #[cfg(feature = "serde")]
        ctx.ok_or_record(self.serde_prepare_struct());
    }

    fn prepare_field(&'a self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let ctx = self.ctx();
        let is_active = !*fctx.skipped();

        if is_active {
            if *fctx.accessor() && fctx.accessor_mode().is_copy() {
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
        let arg_props = ctx.arg_props();

        self.struct_extras();

        let builder_struct = arg_props.builder_struct();

        if *builder_struct {
            let span = builder_struct.final_span();
            let generic_params = ctx.struct_generic_params();
            let builder_ident = arg_props.builder_ident();
            let mut mc = FXFnConstructor::new_associated(format_ident!("builder", span = span));
            let builder_method_doc = arg_props.builder_method_doc().cloned().or_else(|| {
                Some(FXProp::new(
                    vec![parse_quote_spanned![span=> "Creates a new builder for this struct."]],
                    Some(span),
                ))
            });

            ctx.ok_or_record(
                mc.set_span(span)
                    .set_vis(arg_props.builder_struct_visibility())
                    .set_ret_type(quote_spanned! {span=> #builder_ident #generic_params })
                    .set_ret_stmt(quote_spanned! {span=> #builder_ident::new() })
                    .maybe_add_doc(builder_method_doc.as_ref())
                    .and_then(|mc| mc.add_attribute_toks(quote_spanned! {span=> #[inline]})),
            );

            ctx.add_method(mc);
        }

        #[cfg(feature = "serde")]
        self.serde_rewrite_struct();
    }

    fn struct_extras(&'a self) {
        let ctx = self.ctx();
        let cgen = self.struct_codegen();

        self.myself_methods();

        let needs_default = ctx.needs_default();
        let needs_new = ctx.needs_new();

        if *needs_default {
            let span = needs_default.final_span();
            // Generate fn new()
            let new_name = if *needs_new {
                format_ident!("new", span = needs_new.final_span())
            }
            else {
                format_ident!("__fieldx_new", span = span)
            };

            let mut mc = FXFnConstructor::new_associated(new_name);

            ctx.ok_or_record(
                mc.set_span(span)
                    .set_ret_type(cgen.maybe_ref_counted(&quote_spanned![span=> Self]))
                    .set_vis(quote_spanned![span=> pub])
                    .set_ret_stmt(cgen.maybe_ref_counted_create(
                        &quote_spanned![span=> Self],
                        &quote_spanned![span=> ..Self::default()],
                        None,
                    ))
                    .add_attribute_toks(quote_spanned! {span=> #[inline] }),
            );

            ctx.add_method(mc);
        }
    }

    fn myself_methods(&'a self) {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let rc = arg_props.rc();

        if *rc {
            let rc_span = rc.final_span();
            let (rc_type, weak_type) = self.struct_codegen().ref_count_types(rc_span);
            let mut myself_mc = FXFnConstructor::new(arg_props.myself_name().cloned().unwrap());
            let mut downgrade_mc = FXFnConstructor::new(arg_props.myself_downgrade_name().cloned().unwrap());
            let myself_field = arg_props.myself_field_ident();

            ctx.ok_or_record(
                myself_mc
                    .set_span(rc_span)
                    .set_vis(arg_props.rc_visibility().clone())
                    .set_ret_type(quote_spanned![rc_span=> ::std::option::Option<#rc_type<Self>>])
                    .add_statement(quote_spanned![rc_span=> #weak_type::upgrade(&self.#myself_field)])
                    .add_attribute_toks(quote_spanned![rc_span=> #[allow(dead_code)] #[inline(always)]]),
            );
            ctx.ok_or_record(myself_mc.maybe_add_doc(arg_props.rc_doc()));

            ctx.ok_or_record(
                downgrade_mc
                    .set_span(rc_span)
                    .set_vis(arg_props.rc_visibility().clone())
                    .set_ret_type(quote_spanned![rc_span=> #weak_type<Self>])
                    .add_statement(quote_spanned![rc_span=> #weak_type::clone(&self.#myself_field)])
                    .add_attribute_toks(quote_spanned![rc_span=> #[allow(dead_code)] #[inline(always)]]),
            );

            ctx.add_method(myself_mc);
            ctx.add_method(downgrade_mc);
        }
    }

    fn default_impl(&self) {
        let ctx = self.ctx();

        let needs_default = ctx.needs_default();

        if !*needs_default {
            return;
        }

        if let Some(defaults) = ctx.defaults_combined() {
            let span = needs_default.final_span();
            let mut default_impl = FXImplConstructor::new(format_ident!("Default", span = span));
            let mut default_method = FXFnConstructor::new_associated(format_ident!("default", span = span));

            let mut user_struct = ctx.user_struct_mut();
            default_impl
                .set_for_ident(user_struct.ident())
                .set_from_generics(user_struct.generics().clone())
                .set_span(span);
            default_method
                .set_span(span)
                .set_ret_type(quote_spanned! {span=> Self})
                .set_ret_stmt(quote_spanned! {span=> Self { #defaults }});
            default_impl.add_method(default_method);
            user_struct.add_trait_impl(default_impl);
        }
    }

    fn builder_field_ctxs(&self) -> darling::Result<Vec<darling::Result<Rc<FXFieldCtx>>>> {
        let ctx = self.ctx();
        Ok(ctx
            .builder_struct()?
            .field_idents()
            .map(|ident| ctx.ident_field_ctx(&ident))
            .collect())
    }

    fn builder_impl(&'a self) -> darling::Result<()> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let span = arg_props.builder_struct().final_span();
        let cgen = self.struct_codegen();
        let builder_return_type = cgen.maybe_ref_counted(&cgen.builder_return_type());
        let builder_error_type = if let Some(error_type) = arg_props.builder_error_type() {
            quote_spanned![span=> #error_type]
        }
        else {
            quote_spanned! {span=> ::fieldx::error::FieldXError}
        };

        let mut new_method = FXFnConstructor::new_associated(format_ident!("new", span = span));
        let mut build_method = FXFnConstructor::new(format_ident!("build", span = span));

        new_method
            .set_span(span)
            .set_vis(arg_props.builder_struct_visibility())
            .set_ret_type(quote_spanned! {span=> Self})
            .add_attribute_toks(quote_spanned! {span=> #[inline(always)]})?;

        build_method
            .set_span(span)
            .set_self_mut(true)
            .set_vis(arg_props.builder_struct_visibility())
            .set_ret_type(quote_spanned! {span=> ::std::result::Result<#builder_return_type, #builder_error_type>})
            .add_attribute_toks(quote_spanned! {span=> #[inline]})?
            .add_doc(&FXProp::new(
                vec![parse_quote_spanned! {span=> "Builds the struct from the builder object."}],
                Some(span),
            ))?;

        let input_ident = ctx.input_ident();
        let post_build_ident = arg_props.post_build_ident().cloned();

        let mut field_setters = Vec::<TokenStream>::new();
        let mut builder_checkers = vec![];
        let mut fields_new = vec![];

        for fctx in self.builder_field_ctxs()? {
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

                match fgen.field_builder_setter(&fctx) {
                    Ok(fsetter) => field_setters.push(fsetter),
                    Err(err) => {
                        ctx.push_error(err);
                        continue;
                    }
                };

                if let Some(bchecker) = fctx.builder_checker() {
                    builder_checkers.push(bchecker);
                }
            }
            else {
                ctx.push_error(fctx.unwrap_err());
            }
        }

        new_method.set_ret_stmt(quote_spanned! {span=>
            Self {
                #( #fields_new ),*
            }
        });

        let construction = cgen.maybe_ref_counted_create(
            &input_ident.to_token_stream(),
            &quote_spanned! {span=>
                    #(#field_setters,)*
            },
            post_build_ident,
        );

        build_method.add_statement(quote_spanned! {span=> #( #builder_checkers );* });
        build_method.set_ret_stmt(quote_spanned! {span=> Ok(#construction) });

        let mut bsc = ctx.builder_struct_mut()?;
        let bic = bsc.struct_impl_mut();
        bic.add_method(new_method);
        bic.add_method(build_method);

        Ok(())
    }

    fn builder_struct(&'a self) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let builder_struct = arg_props.builder_struct();

        Ok(if *builder_struct {
            self.builder_impl()?;
            ctx.builder_struct_mut()?
                .maybe_add_doc(arg_props.builder_doc())?
                .to_token_stream()
        }
        else {
            quote![]
        })
    }

    fn finalize(&'a self) -> TokenStream {
        let ctx = self.ctx();
        // This is the case where we better point to the `fxstruct` attribute itself.
        let span = Span::call_site(); // ctx.input().ident().span();

        // Make sure the Default trait is implemented if needed.
        self.default_impl();

        // ctx.add_attr(self.derive_toks(&self.derive_traits()));

        ctx.user_struct_mut()
            .struct_impl_mut()
            // .add_attributes(ctx.all_attrs().iter())
            .maybe_add_attributes(ctx.args().attributes_impl().as_ref().map(|a| a.iter()));

        let copyables = ctx.copyable_types();
        if !copyables.is_empty() {
            let copyables: Vec<TokenStream> = copyables
                .iter()
                .map(|ct| {
                    quote_spanned! {ct.span()=> __field_implements_copy::<#ct>()}
                })
                .collect();

            let mut fcv_fn = FXFnConstructor::new_associated(format_ident!("__fieldx_copy_validation", span = span));

            ctx.ok_or_record(
                fcv_fn
                    .set_span(span)
                    .add_statement(quote_spanned! {span=> fn __field_implements_copy<T: ?Sized + Copy>() {} })
                    .add_statement(quote_spanned! {span=> #( #copyables; )* })
                    .add_attribute_toks(quote_spanned! {span=> #[allow(dead_code)] }),
            );

            ctx.user_struct_mut().struct_impl_mut().add_method(fcv_fn);
        }

        let mut fxstruct_trait = FXImplConstructor::new(format_ident!("FXStruct", span = span));
        fxstruct_trait
            .set_span(span)
            .set_for_ident(ctx.input_ident())
            .set_from_generics(Some(ctx.input().generics().clone()));
        ctx.user_struct_mut().add_trait_impl(fxstruct_trait);

        let builder_struct = ctx.ok_or_empty(self.builder_struct());
        let user_struct = ctx.user_struct().to_token_stream();
        #[cfg(feature = "serde")]
        let shadow_struct = ctx.ok_or_empty(self.serde_finalize());
        #[cfg(not(feature = "serde"))]
        let shadow_struct = quote![];

        let struct_impl = quote_spanned! {span=>
            #[allow(unused_imports)]
            use ::fieldx::traits::*;

            #user_struct
            #builder_struct
            #shadow_struct
        };

        ctx.tokens_extend(struct_impl);
        ctx.finalize()
    }
}
