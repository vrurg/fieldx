use crate::{helper::*, util::args::FXSArgs, FXInputReceiver};
pub use darling::{Error as DError, Result as DResult};
use enum_dispatch::enum_dispatch;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{spanned, spanned::Spanned, Expr, Ident, Lit};

use context::{FXCodeGenCtx, FXFieldCtx};
mod context;
mod nonsync;
mod sync;

#[enum_dispatch]
pub trait FXCGen<'f> {
    // Actual code producers
    fn field_accessor(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_accessor_mut(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_builder_setter(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_reader(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_writer(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_setter(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_clearer(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_predicate(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_default_wrap(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;

    fn struct_extras(&self);

    // Helper methods
    fn add_field_decl(&self, field: TokenStream);
    fn add_defaults_decl(&self, defaults: TokenStream);
    fn add_method_decl(&self, method: TokenStream);
    fn add_initializer_decl(&self, initializer: TokenStream);
    fn add_builder_decl(&self, builder_method: TokenStream);
    fn add_builder_field_decl(&self, builder_field: TokenStream);
    fn add_builder_field_ctx(&self, field_ctx: FXFieldCtx<'f>);

    fn ctx(&self) -> &FXCodeGenCtx;
    fn type_tokens<'s>(&'s self, field_ctx: &'s FXFieldCtx) -> &'s TokenStream;
    // fn type_tokens_mut<'s>(&'s self, field_ctx: &'s FXFieldCtx) -> &'s TokenStream;

    fn methods_combined(&self) -> TokenStream;
    fn fields_combined(&self) -> TokenStream;
    fn defaults_combined(&self) -> TokenStream;
    fn initializers_combined(&self) -> TokenStream;
    fn builder_fields_combined(&self) -> TokenStream;
    fn builders_combined(&self) -> TokenStream;
    fn builder_fields_ctx(&'f self) -> std::cell::Ref<Vec<FXFieldCtx<'f>>>;
    fn builder_trait(&self) -> TokenStream;

    fn needs_builder_struct(&self) -> bool {
        self.ctx().needs_builder_struct().unwrap_or(false)
    }

    fn field_needs_into(&self, field_ctx: &FXFieldCtx) -> bool {
        if field_ctx.needs_into().is_some() {
            field_ctx.is_into()
        }
        else {
            self.ctx().needs_into().unwrap_or(false)
        }
    }

    fn field_needs_builder(&self, field_ctx: &FXFieldCtx) -> bool {
        if let Some(needs) = field_ctx.needs_builder() {
            needs
        }
        else {
            self.needs_builder_struct()
        }
    }

    // Common implementations
    fn input(&self) -> &FXInputReceiver {
        &self.ctx().input()
    }

    fn ok_or(&self, outcome: DResult<TokenStream>) -> TokenStream {
        outcome.unwrap_or_else(|err| {
            self.ctx().push_error(err);
            quote![]
        })
    }

    fn helper_name(
        &self,
        field_ctx: &FXFieldCtx,
        helper: &Option<FXHelper>,
        default_pfx: Option<&str>,
        default_sfx: Option<&str>,
    ) -> Option<Ident> {
        match helper {
            Some(ref h) => match h.value() {
                FXHelperKind::Flag(ref flag) => {
                    if !flag {
                        return None;
                    }
                    let helper_base_name = field_ctx.helper_base_name()?;
                    Some(format_ident![
                        "{}{}{}",
                        if let Some(pfx) = default_pfx {
                            [pfx, "_"].join("")
                        }
                        else {
                            "".to_string()
                        },
                        helper_base_name,
                        if let Some(sfx) = default_sfx {
                            ["_", sfx].join("")
                        }
                        else {
                            "".to_string()
                        }
                    ])
                }
                FXHelperKind::Name(ref name) => {
                    if name.is_empty() {
                        None
                    }
                    else {
                        Some(format_ident!("{}", name.clone()))
                    }
                }
            },
            None => None,
        }
    }

    fn helper_name_tok(
        &self,
        field_ctx: &FXFieldCtx,
        helper: &Option<FXHelper>,
        helper_name: &str,
        default_pfx: Option<&str>,
        default_sfx: Option<&str>,
    ) -> DResult<TokenStream> {
        if let Some(ref helper_ident) = self.helper_name(field_ctx, helper, default_pfx, default_sfx) {
            Ok(helper_ident.to_token_stream())
        }
        else {
            let err = DError::custom(format!(
                "Expected to have {} helper method name {}",
                helper_name,
                field_ctx.for_ident_str()
            ));
            if let Some(ref fxh) = helper {
                Err(err.with_span(fxh))
            }
            else {
                Err(err)
            }
        }
    }

    fn accessor_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        let aname = self.helper_name_tok(field_ctx, field_ctx.accessor(), "accessor", None, None);
        if aname.is_err() && field_ctx.needs_accessor(self.ctx().is_sync()) {
            if let Some(helper_base_name) = field_ctx.helper_base_name() {
                return Ok(format_ident!("{}", helper_base_name).to_token_stream());
            }
        }
        aname
    }

    fn accessor_mut_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.accessor_mut(), "accessor_mut", None, Some("mut"))
    }

    fn builder_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.builder(), "builder", None, None)
    }

    fn lazy_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.lazy(), "lazy builder", Some("build"), None)
    }

    fn setter_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.setter(), "setter", Some("set"), None)
    }

    fn clearer_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.clearer(), "clearer", Some("clear"), None)
    }

    fn predicate_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.predicate(), "predicate", Some("has"), None)
    }

    fn reader_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.reader(), "reader", Some("read"), None)
    }

    fn writer_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.writer(), "writer", Some("write"), None)
    }

    fn generic_params(&self) -> TokenStream {
        let generic_idents = self.ctx().input().generic_param_idents();

        if generic_idents.len() > 0 {
            quote![< #( #generic_idents ),* >]
        }
        else {
            quote![]
        }
    }

    fn field_initializer(&self, _field_ctx: &FXFieldCtx) {}

    fn field_default_value(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        let field = field_ctx.field();

        if let Some(def_meta) = field_ctx.default() {
            let val_expr = &def_meta.require_name_value()?.value;
            let mut is_str = false;
            let span = (field_ctx.default().as_ref().unwrap() as &dyn spanned::Spanned).span();

            if let Expr::Lit(lit_val) = val_expr {
                if let Lit::Str(_) = lit_val.lit {
                    is_str = true;
                }
            }

            if is_str {
                Ok(quote_spanned! [span=> ::std::string::String::from(#val_expr) ])
            }
            else {
                Ok(quote_spanned! [span=> #val_expr ])
            }
        }
        else if field.is_lazy() {
            Ok(quote![])
        }
        else {
            Ok(quote_spanned! [field.span()=> ::std::default::Default::default() ])
        }
    }

    fn field_decl(&self, field_ctx: FXFieldCtx<'f>) {
        let attrs = field_ctx.attrs();
        let vis = field_ctx.vis();

        let ty_tok = self.type_tokens(&field_ctx);
        // No check for None is needed because we're only applying to named structs.
        let ident = field_ctx.ident_tok();

        self.add_field_decl(quote_spanned! [*field_ctx.span()=>
            #( #attrs )*
            #vis #ident: #ty_tok
        ]);

        self.field_initializer(&field_ctx);
        self.field_default(&field_ctx);
        self.field_methods(field_ctx);
    }

    fn field_methods(&self, field_ctx: FXFieldCtx<'f>) {
        self.add_method_decl(self.ok_or(self.field_accessor(&field_ctx)));
        self.add_method_decl(self.ok_or(self.field_accessor_mut(&field_ctx)));
        self.add_method_decl(self.ok_or(self.field_reader(&field_ctx)));
        self.add_method_decl(self.ok_or(self.field_writer(&field_ctx)));
        self.add_method_decl(self.ok_or(self.field_setter(&field_ctx)));
        self.add_method_decl(self.ok_or(self.field_clearer(&field_ctx)));
        self.add_method_decl(self.ok_or(self.field_predicate(&field_ctx)));
        if self.needs_builder_struct() {
            self.add_builder_decl(self.ok_or(self.field_builder(&field_ctx)));
            self.add_builder_field_decl(self.ok_or(self.field_builder_field(&field_ctx)));
            self.add_builder_field_ctx(field_ctx);
        }
    }

    fn rewrite_struct(&'f self) {
        // If builder requirement is not set explicitly with fxstruct attribute then check out if any field is asking
        // for it.
        if self.ctx().needs_builder_struct().is_none() {
            for field in self.input().fields() {
                if let Some(needs) = field.needs_builder() {
                    if needs {
                        self.ctx().require_builder();
                    }
                }
            }
        }

        for field in self.input().fields() {
            let field_ctx = FXFieldCtx::<'f>::new(field);
            self.field_decl(field_ctx);
        }

        self.struct_extras();

        if self.needs_builder_struct() {
            let builder_ident = self.builder_ident();
            let generic_params = self.generic_params();
            self.add_method_decl(quote![
                #[inline]
                pub fn builder() -> #builder_ident #generic_params {
                    #builder_ident::default()
                }
            ])
        }
    }

    fn field_default(&self, field_ctx: &FXFieldCtx) {
        let def_tok = self.ok_or(self.field_default_wrap(field_ctx));
        let ident = field_ctx.ident_tok();
        self.add_defaults_decl(quote! [ #ident: #def_tok ])
    }

    fn simple_field_build_setter(&self, field_ctx: &FXFieldCtx, field_ident: &TokenStream, span: &Span) -> TokenStream {
        let field_name = field_ident.to_string();
        let alternative = if field_ctx.has_default() {
            self.ok_or(self.field_default_wrap(field_ctx))
        }
        else {
            quote![
                return ::std::result::Result::Err(
                    ::std::convert::Into::into(
                        ::fieldx::errors::UninitializedFieldError::new(#field_name) )
                )
            ]
        };

        quote_spanned![*span=>
            #field_ident: if let ::std::option::Option::Some(field_manual_value) = self.#field_ident.take() {
                field_manual_value
            }
            else {
                #alternative
            }
        ]
    }

    fn default_impl(&self) -> TokenStream {
        let defaults = self.defaults_combined();
        let ctx = self.ctx();
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

    fn builder_ident(&self) -> TokenStream {
        let ident = self.ctx().input_ident();
        format_ident!("{}{}", ident, "Builder").to_token_stream()
    }

    fn field_builder_field(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if fctx.needs_builder().unwrap_or(true) {
            let ident = fctx.ident_tok();
            let span = *fctx.span();
            let ty = fctx.ty();
            if fctx.is_ignorable() {
                Ok(quote_spanned![span=> #ident: #ty])
            }
            else {
                Ok(quote_spanned![span=> #ident: ::std::option::Option<#ty>])
            }
        }
        else {
            Ok(quote![])
        }
    }

    fn field_builder(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if !fctx.is_ignorable() && fctx.builder().as_ref().unwrap_or(&FXHelper::from(true)).is_true() {
            let ident = fctx.ident_tok();
            let builder_name: TokenStream = self
                .builder_name(fctx)
                .unwrap_or(format_ident!("{}", fctx.helper_base_name().expect("Field name")).to_token_stream());
            let span = *fctx.span();
            let ty = fctx.ty();
            if self.field_needs_into(fctx) {
                Ok(quote_spanned![span=>
                    pub fn #builder_name<FXVALINTO: ::std::convert::Into<#ty>>(&mut self, value: FXVALINTO) -> &mut Self {
                        self.#ident = ::std::option::Option::Some(value.into());
                        self
                    }
                ])
            }
            else {
                Ok(quote_spanned![span=>
                    pub fn #builder_name(&mut self, value: #ty) -> &mut Self {
                        self.#ident = ::std::option::Option::Some(value);
                        self
                    }
                ])
            }
        }
        else {
            Ok(quote![])
        }
    }

    fn builder_struct(&'f self) -> TokenStream {
        if self.needs_builder_struct() {
            let builder_ident = self.builder_ident();
            let builder_fields = self.builder_fields_combined();
            let builder_impl = self.builder_impl();
            let generics = self.ctx().input().generics();
            let where_clause = &generics.where_clause;
            let span = proc_macro2::Span::call_site();
            quote_spanned![span=>
                #[derive(Default)]
                struct #builder_ident #generics
                #where_clause
                {
                    #builder_fields
                }

                #builder_impl
            ]
        }
        else {
            quote![]
        }
    }

    #[inline]
    fn wrap_construction(&self, construction: TokenStream) -> TokenStream {
        construction
    }

    #[inline]
    fn builder_return_type(&self) -> TokenStream {
        let builder_ident = self.ctx().input_ident();
        let generic_params = self.generic_params();
        quote![#builder_ident #generic_params]
    }

    fn builder_impl(&'f self) -> TokenStream {
        let ctx = self.ctx();
        let builder_ident = self.builder_ident();
        let builders = self.builders_combined();
        let input_ident = ctx.input_ident();
        let generics = ctx.input().generics();
        let where_clause = &generics.where_clause;
        let generic_params = self.generic_params();
        let builder_return_type = self.builder_return_type();
        let builder_trait = self.builder_trait();

        let mut field_setters = Vec::<TokenStream>::new();
        let mut use_default = false;
        for fctx in self.builder_fields_ctx().iter() {
            let fsetter = self.ok_or(self.field_builder_setter(&fctx));
            if fsetter.is_empty() {
                use_default = true;
            }
            else {
                field_setters.push(fsetter);
            }
        }

        let default_initializer = if use_default {
            let comma = if field_setters.is_empty() { quote![] } else { quote![,] };
            quote![#comma ..::core::default::Default::default()]
        }
        else {
            quote![]
        };

        let construction = self.wrap_construction(quote![
            #input_ident {
                #(#field_setters),*
                #default_initializer
            }
        ]);

        quote![
            #[allow(dead_code)]
            impl #generics #builder_ident #generic_params
            #where_clause
            {
                #builders
            }

            #[allow(dead_code)]
            impl #generics #builder_trait for #builder_ident #generic_params
            #where_clause
            {
                type TargetStruct = #input_ident #generic_params;
                fn build(&mut self) -> ::std::result::Result<#builder_return_type, ::fieldx::errors::UninitializedFieldError> {
                    Ok(#construction)
                }
            }
        ]
    }

    fn finalize(&'f self) -> TokenStream {
        let input = self.input();

        let &FXInputReceiver {
            ref attrs,
            ref vis,
            ref ident,
            ref generics,
            ..
        } = input;

        let methods = self.methods_combined();
        let fields = self.fields_combined();
        let default = self.default_impl();
        let builder_struct = self.builder_struct();
        let where_clause = &generics.where_clause;
        let generic_params = self.generic_params();

        self.ctx().tokens_extend(quote! [
            use ::fieldx::traits::*;

            #( #attrs )*
            #vis struct #ident #generics
            #where_clause
            {
                #fields
            }

            impl #generics FXStruct for #ident #generic_params #where_clause {}

            impl #generics #ident #generics #where_clause {
                #methods
            }

            #default

            #builder_struct
        ]);
        self.ctx().finalize()
    }
}

// FieldX Code Generator – FXCG
#[enum_dispatch(FXCGen)]
enum FXCG<'f> {
    NonSync(nonsync::FXCodeGen<'f>),
    Sync(sync::FXCodeGen<'f>),
}

pub struct FXRewriter<'f> {
    generator: FXCG<'f>,
}

impl<'f> FXRewriter<'f> {
    pub fn new(input: FXInputReceiver, args: &FXSArgs) -> Self {
        let ctx = FXCodeGenCtx::new(input, args);

        let generator: FXCG = if ctx.is_sync() {
            sync::FXCodeGen::new(ctx).into()
        }
        else {
            nonsync::FXCodeGen::new(ctx).into()
        };

        Self { generator }
    }

    pub fn rewrite(&'f mut self) -> TokenStream {
        self.generator.rewrite_struct();
        self.generator.finalize()
    }
}
