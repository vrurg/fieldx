use crate::{helper::*, util::args::FXSArgs, FXInputReceiver};
pub use darling::{Error as DError, Result as DResult};
use enum_dispatch::enum_dispatch;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::cell::Ref;
use syn::{spanned, spanned::Spanned, Expr, Ident, Lit};

use context::{FXCodeGenCtx, FXFieldCtx};
mod context;
mod nonsync;
mod sync;

#[enum_dispatch]
pub(crate) trait FXCGen<'f> {
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

    fn add_field_decl(&self, field: TokenStream);
    fn add_defaults_decl(&self, defaults: TokenStream);
    fn add_method_decl(&self, method: TokenStream);
    fn add_initializer_decl(&self, initializer: TokenStream);
    fn add_builder_decl(&self, builder_method: TokenStream);
    fn add_builder_field_decl(&self, builder_field: TokenStream);
    fn add_builder_field_ctx(&self, field_ctx: FXFieldCtx<'f>);
    fn check_for_impl_copy(&self, field_ctx: &FXFieldCtx);

    fn ctx(&self) -> &FXCodeGenCtx;
    fn type_tokens<'s>(&'s self, field_ctx: &'s FXFieldCtx) -> &'s TokenStream;
    fn copyable_types(&self) -> Ref<Vec<syn::Type>>;

    fn methods_combined(&self) -> TokenStream;
    fn fields_combined(&self) -> TokenStream;
    fn defaults_combined(&self) -> TokenStream;
    fn initializers_combined(&self) -> TokenStream;
    fn builder_fields_combined(&self) -> TokenStream;
    fn builders_combined(&self) -> TokenStream;
    fn builder_fields_ctx(&'f self) -> std::cell::Ref<Vec<FXFieldCtx<'f>>>;
    fn builder_trait(&self) -> TokenStream;

    #[inline]
    fn needs_builder_struct(&self) -> bool {
        self.ctx().needs_builder_struct().unwrap_or(false)
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
        helper: Option<&impl FXHelperTrait>,
        helper_name: &str,
        default_pfx: Option<&str>,
        default_sfx: Option<&str>,
    ) -> DResult<Ident> {
        if let Some(ref h) = helper {
            if let Some(ref name) = h.rename() {
                if !name.is_empty() {
                    return Ok(format_ident!("{}", name));
                }
            }
        }

        let helper_base_name = field_ctx.helper_base_name().ok_or(
            darling::Error::custom(format!(
                "This field doesn't have a name I can use to name {} helper",
                helper_name
            ))
            .with_span(field_ctx.field()),
        )?;
        Ok(format_ident![
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

    fn helper_name_tok(
        &self,
        field_ctx: &FXFieldCtx,
        helper: &Option<FXNestingAttr<impl FXHelperTrait + FromNestAttr>>,
        helper_name: &str,
        default_pfx: Option<&str>,
        default_sfx: Option<&str>,
    ) -> DResult<TokenStream> {
        Ok(self
            .helper_name(
                field_ctx,
                helper.as_ref().map(|h| &**h),
                helper_name,
                default_pfx,
                default_sfx,
            )?
            .to_token_stream())
    }

    fn accessor_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.accessor(), "accessor", None, None)
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

    fn into_toks(&self, field_ctx: &FXFieldCtx, use_into: bool) -> (TokenStream, TokenStream, TokenStream) {
        let ty = field_ctx.ty();
        if use_into {
            (
                quote![<FXVALINTO: ::std::convert::Into<#ty>>],
                quote![FXVALINTO],
                quote![.into()],
            )
        }
        else {
            (quote![], quote![#ty], quote![])
        }
    }

    fn field_initializer(&self, _field_ctx: &FXFieldCtx) {}

    fn field_default_value(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        let field = field_ctx.field();

        if let Some(def_meta) = field_ctx.default_value() {
            let val_expr = &def_meta.require_name_value()?.value;
            let mut is_str = false;
            let span = (field_ctx.default_value().as_ref().unwrap() as &dyn spanned::Spanned).span();

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
        else if field_ctx.is_lazy() {
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

        if field_ctx.needs_accessor() && field_ctx.is_copy() {
            self.check_for_impl_copy(&field_ctx);
        }

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
        let ctx = self.ctx();
        // If builder requirement is not set explicitly with fxstruct attribute then check out if any field is asking
        // for it.
        if ctx.needs_builder_struct().is_none() {
            for field in self.input().fields() {
                if let Some(needs) = field.needs_builder() {
                    if needs {
                        self.ctx().require_builder();
                    }
                }
            }
        }

        for field in self.input().fields() {
            let field_ctx = FXFieldCtx::<'f>::new(field, ctx);
            self.field_decl(field_ctx);
        }

        self.struct_extras();

        if self.needs_builder_struct() {
            let builder_ident = self.builder_ident();
            let generic_params = self.generic_params();
            let vis = self.ctx().input().vis();
            self.add_method_decl(quote![
                #[inline]
                #vis fn builder() -> #builder_ident #generic_params {
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
        let alternative = if field_ctx.has_default_value() {
            self.ok_or(self.field_default_wrap(field_ctx))
        }
        else {
            quote![
                return ::std::result::Result::Err(
                    ::std::convert::Into::into(
                        ::fieldx::errors::FieldXError::uninitialized_field(#field_name.into()) )
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
        if fctx.needs_builder() {
            let ident = fctx.ident_tok();
            let span = *fctx.span();
            let ty = fctx.ty();
            let attributes = fctx.builder_attributes();
            if fctx.is_ignorable() {
                Ok(quote_spanned![span=> #attributes #ident: #ty])
            }
            else {
                Ok(quote_spanned![span=> #attributes #ident: ::std::option::Option<#ty>])
            }
        }
        else {
            Ok(quote![])
        }
    }

    fn field_builder(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if !fctx.is_ignorable() && fctx.needs_builder() {
            let ident = fctx.ident_tok();
            let builder_name: TokenStream = self
                .builder_name(fctx)
                .unwrap_or(format_ident!("{}", fctx.helper_base_name().expect("Field name")).to_token_stream());
            let span = *fctx.span();
            let (gen_params, val_type, into_tok) = self.into_toks(fctx, fctx.is_builder_into());
            let attributes = fctx.builder_fn_attributes();
            Ok(quote_spanned![span=>
                #attributes
                pub fn #builder_name #gen_params(&mut self, value: #val_type) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(value #into_tok);
                    self
                }
            ])
        }
        else {
            Ok(quote![])
        }
    }

    fn builder_struct(&'f self) -> TokenStream {
        if self.needs_builder_struct() {
            let ctx = self.ctx();
            let args = ctx.args();
            let builder_ident = self.builder_ident();
            let builder_fields = self.builder_fields_combined();
            let builder_impl = self.builder_impl();
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;
            let span = proc_macro2::Span::call_site();
            let vis = ctx.input().vis();
            let attributes = args.builder_attributes();
            quote_spanned![span=>
                #[derive(Default)]
                #attributes
                #vis struct #builder_ident #generics
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
        let vis = ctx.input().vis();
        let builder_ident = self.builder_ident();
        let builders = self.builders_combined();
        let input_ident = ctx.input_ident();
        let generics = ctx.input().generics();
        let where_clause = &generics.where_clause;
        let generic_params = self.generic_params();
        let builder_return_type = self.builder_return_type();
        let attributes = ctx.args().builder_impl_attributes();

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
            #attributes
            impl #generics #builder_ident #generic_params
            #where_clause
            {
                #builders
                #vis fn build(&mut self) -> ::std::result::Result<#builder_return_type, ::fieldx::errors::FieldXError> {
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

        let copyables = self.copyable_types();
        let copyable_validation = if !copyables.is_empty() {
            let copyables: Vec<TokenStream> = copyables.iter().map(|ct| ct.to_token_stream()).collect();
            Some(quote![
                const _: fn() = || {
                    fn field_implements_copy<T: ?Sized + Copy>() {}
                    #( field_implements_copy::<#copyables>(); )*
                };
            ])
        }
        else {
            None
        };

        self.ctx().tokens_extend(quote! [
            use ::fieldx::traits::*;

            #copyable_validation

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
    pub fn new(input: FXInputReceiver, args: FXSArgs) -> Self {
        let ctx = FXCodeGenCtx::new(input, args);

        let generator: FXCG = if ctx.args().is_sync() {
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