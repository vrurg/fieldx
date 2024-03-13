use std::default;

use crate::{fields::FXField, helper::*, util::args::FXSArgs, FXInputReceiver};
pub use darling::{Error as DError, Result as DResult};
use enum_dispatch::enum_dispatch;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{spanned, spanned::Spanned, Expr, Ident, Lit};

use context::{FXCodeGenCtx, FXFieldCtx};
mod context;
mod nonsync;
mod sync;

#[enum_dispatch]
pub trait FXCGen {
    // Actual code producers
    fn field_accessor(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_reader(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_writer(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_setter(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_clearer(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_predicate(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;
    fn field_default_wrap(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream>;

    fn field_initializer(&self, field_ctx: &FXFieldCtx);
    fn struct_extras(&self);

    // Helper methods
    fn add_field_decl(&self, field: TokenStream);
    fn add_defaults_decl(&self, defaults: TokenStream);
    fn add_method_decl(&self, method: TokenStream);
    fn add_initializer_decl(&self, initializer: TokenStream);

    fn ctx(&self) -> &FXCodeGenCtx;
    fn type_tokens<'s>(&'s self, field_ctx: &'s FXFieldCtx) -> &'s TokenStream;

    fn methods_combined(&self) -> TokenStream;
    fn fields_combined(&self) -> TokenStream;
    fn defaults_combined(&self) -> TokenStream;
    fn initializers_combined(&self) -> TokenStream;

    // Common implementations
    fn input(&self) -> &FXInputReceiver {
        &self.ctx().input
    }

    fn ok_or(&self, for_elem: Option<&impl ToTokens>, err: DResult<TokenStream>) -> TokenStream {
        err.unwrap_or_else(|err| {
            self.ctx().push_error(err);
            if let Some(elem) = for_elem {
                elem.to_token_stream()
            }
            else {
                TokenStream::new()
            }
        })
    }

    fn helper_name(
        &self,
        field_ctx: &FXFieldCtx,
        helper: &Option<FXHelper>,
        default_pfx: Option<&str>,
    ) -> Option<Ident> {
        match helper {
            Some(ref h) => match h.value() {
                FXHelperKind::Flag(ref flag) => {
                    let helper_base_name = field_ctx.helper_base_name()?;
                    if let Some(pfx) = default_pfx {
                        Some(format_ident!("{}_{}", pfx, helper_base_name))
                    }
                    else {
                        Some(format_ident!("{}", helper_base_name))
                    }
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
        default_pfx: Option<&str>,
        helper_name: &str,
    ) -> DResult<TokenStream> {
        if let Some(ref helper_ident) = self.helper_name(field_ctx, helper, default_pfx) {
            Ok(helper_ident.to_token_stream())
        }
        else {
            let err = DError::custom(format!(
                "Expected to have a method-helper name for {}{}",
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
        if let Some(ahelper) = field_ctx.accessor() {
            self.helper_name_tok(field_ctx, ahelper, None, "accessor")
        }
        else {
            let ident = field_ctx.ident_tok();
            Ok(quote! [#ident])
        }
    }

    fn setter_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.setter(), Some("set"), "setter")
    }

    fn clearer_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.clearer(), Some("clear"), "clearer")
    }

    fn predicate_name(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        self.helper_name_tok(field_ctx, field_ctx.predicate(), Some("has"), "predicate")
    }

    fn field_default_value(&self, field_ctx: &FXFieldCtx) -> DResult<TokenStream> {
        let field = field_ctx.field();

        if field.is_lazy() && field_ctx.default().is_some() {
            Err(DError::custom(format!(
                "Argument 'default' must not be used when 'lazy' is enabled{}",
                field_ctx.for_ident_str()
            ))
            .with_span(field_ctx.default().as_ref().unwrap())
            .span_note(field_ctx.lazy(), "The 'lazy' argument is declared here")
            .help("'lazy' and 'default' arguments are mutually exclusive; try removing either one"))
        }
        else if let Some(def_meta) = field_ctx.default() {
            let val_expr = &def_meta.require_name_value()?.value;
            let mut is_str = false;
            let span = (field_ctx.default().as_ref().unwrap() as &dyn spanned::Spanned).span();

            if let Expr::Lit(lit_val) = val_expr {
                if let Lit::Str(_) = lit_val.lit {
                    is_str = true;
                }
            }

            if is_str {
                Ok(quote_spanned! [span=> String::from(#val_expr) ])
            }
            else {
                Ok(quote_spanned! [span=> #val_expr ])
            }
        }
        else {
            Ok(quote_spanned! [field.span()=> Default::default() ])
        }
    }

    fn field_decl(&self, field_ctx: &FXFieldCtx) {
        let attrs = field_ctx.attrs();
        let vis = field_ctx.vis();

        let ty_tok = self.type_tokens(field_ctx);
        // No check for None is needed because we're only applying to named structs.
        let ident = field_ctx.ident_tok();

        self.add_field_decl(quote_spanned! [*field_ctx.span()=>
            #( #attrs )*
            #vis #ident: #ty_tok
        ]);

        self.field_methods(field_ctx);
        self.field_initializer(field_ctx);
        self.field_default(field_ctx);
    }

    fn field_methods(&self, field_ctx: &FXFieldCtx) {
        let field = field_ctx.field();

        self.add_method_decl(self.ok_or(Some(field), self.field_accessor(field_ctx)));
        self.add_method_decl(self.ok_or(Some(field), self.field_reader(field_ctx)));
        self.add_method_decl(self.ok_or(Some(field), self.field_writer(field_ctx)));
        self.add_method_decl(self.ok_or(Some(field), self.field_setter(field_ctx)));
        self.add_method_decl(self.ok_or(Some(field), self.field_clearer(field_ctx)));
        self.add_method_decl(self.ok_or(Some(field), self.field_predicate(field_ctx)));
    }

    fn rewrite_struct(&self) {
        for field in self.input().fields() {
            let field_ctx = context::FXFieldCtx::new(field);
            self.field_decl(&field_ctx);
        }
        self.struct_extras();
    }

    fn field_default(&self, field_ctx: &FXFieldCtx) {
        let def_tok = self.ok_or(None::<&FXField>, self.field_default_wrap(field_ctx));
        let ident = field_ctx.ident_tok();
        self.add_defaults_decl(quote! [ #ident: #def_tok ])
    }

    fn default_impl(&self) -> TokenStream {
        let defaults = self.defaults_combined();
        let ident = &self.ctx().input.ident;
        if !defaults.is_empty() {
            quote! [
                impl Default for #ident {
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

    fn finalize(&self) -> TokenStream {
        let input = self.input();

        let &FXInputReceiver {
            ref attrs,
            ref vis,
            ref ident,
            ..
        } = input;

        let methods = self.methods_combined();
        let fields = self.fields_combined();
        let default = self.default_impl();

        self.ctx().tokens_extend(quote! [
            #( #attrs )*
            #vis struct #ident {
                #fields
            }

            impl #ident {
                #methods
            }

            #default
        ]);
        self.ctx().finalize()
    }
}

// FieldX Code Generator – FXCG
#[enum_dispatch(FXCGen)]
enum FXCG {
    NonSync(nonsync::FXCodeGen),
    Sync(sync::FXCodeGen),
}

pub struct FXRewriter {
    generator: FXCG,
}

impl FXRewriter {
    pub fn new(input: FXInputReceiver, args: &FXSArgs) -> Self {
        let ctx = FXCodeGenCtx::new(input, args);

        let generator: FXCG = if ctx.is_sync {
            sync::FXCodeGen::new(ctx).into()
        }
        else {
            nonsync::FXCodeGen::new(ctx).into()
        };

        Self { generator }
    }

    pub fn rewrite(&mut self) -> TokenStream {
        self.generator.rewrite_struct();
        self.generator.finalize()
    }
}
