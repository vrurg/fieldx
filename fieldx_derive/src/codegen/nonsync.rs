#[cfg(feature = "serde")]
use super::FXCGenSerde;
use super::{FXAccessorMode, FXCGen, FXCGenContextual, FXHelperKind};
use crate::{
    codegen::context::{FXCodeGenCtx, FXFieldCtx},
    // util::fxtrace,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};
use syn::spanned::Spanned;

pub(crate) struct FXCodeGen<'f> {
    ctx:                 FXCodeGenCtx,
    field_ctx_table:     RefCell<HashMap<syn::Ident, FXFieldCtx<'f>>>,
    field_toks:          RefCell<Vec<TokenStream>>,
    default_toks:        RefCell<Vec<TokenStream>>,
    method_toks:         RefCell<Vec<TokenStream>>,
    builder_field_toks:  RefCell<Vec<TokenStream>>,
    builder_toks:        RefCell<Vec<TokenStream>>,
    builder_field_ident: RefCell<Vec<syn::Ident>>,
    // List of types to be verified for implementing Copy trait
    copyable_types:      RefCell<Vec<syn::Type>>,
    #[cfg(feature = "serde")]
    shadow_field_toks:   RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_default_toks: RefCell<Vec<TokenStream>>,
}

impl<'f> FXCodeGen<'f> {
    pub fn new(ctx: FXCodeGenCtx) -> Self {
        Self {
            ctx,
            field_ctx_table: RefCell::new(HashMap::new()),
            field_toks: RefCell::new(vec![]),
            default_toks: RefCell::new(vec![]),
            method_toks: RefCell::new(vec![]),
            builder_field_toks: RefCell::new(vec![]),
            builder_field_ident: RefCell::new(vec![]),
            builder_toks: RefCell::new(vec![]),
            copyable_types: RefCell::new(vec![]),
            #[cfg(feature = "serde")]
            shadow_field_toks: RefCell::new(vec![]),
            #[cfg(feature = "serde")]
            shadow_default_toks: RefCell::new(vec![]),
        }
    }
}

impl<'f> FXCGenContextual<'f> for FXCodeGen<'f> {
    #[inline(always)]
    fn ctx(&self) -> &FXCodeGenCtx {
        &self.ctx
    }

    #[inline(always)]
    fn field_ctx_table(&'f self) -> Ref<HashMap<syn::Ident, FXFieldCtx<'f>>> {
        self.field_ctx_table.borrow()
    }

    #[inline(always)]
    fn field_ctx_table_mut(&'f self) -> RefMut<HashMap<syn::Ident, FXFieldCtx<'f>>> {
        self.field_ctx_table.borrow_mut()
    }

    #[inline(always)]
    fn builder_field_ident(&self) -> &RefCell<Vec<syn::Ident>> {
        &self.builder_field_ident
    }

    fn copyable_types(&self) -> std::cell::Ref<Vec<syn::Type>> {
        self.copyable_types.borrow()
    }

    #[cfg(feature = "serde")]
    fn shadow_fields(&self) -> std::cell::Ref<Vec<TokenStream>> {
        self.shadow_field_toks.borrow()
    }

    #[cfg(feature = "serde")]
    fn shadow_defaults(&self) -> std::cell::Ref<Vec<TokenStream>> {
        self.shadow_default_toks.borrow()
    }

    fn add_field_decl(&self, field: TokenStream) {
        self.field_toks.borrow_mut().push(field);
    }

    fn add_defaults_decl(&self, defaults: TokenStream) {
        self.default_toks.borrow_mut().push(defaults);
    }

    fn add_method_decl(&self, method: TokenStream) {
        self.method_toks.borrow_mut().push(method);
    }

    fn add_builder_decl(&self, builder: TokenStream) {
        if !builder.is_empty() {
            self.builder_toks.borrow_mut().push(builder);
        }
    }

    fn add_builder_field_decl(&self, builder_field: TokenStream) {
        if !builder_field.is_empty() {
            self.builder_field_toks.borrow_mut().push(builder_field);
        }
    }

    fn add_builder_field_ident(&self, field_ident: syn::Ident) {
        self.builder_field_ident.borrow_mut().push(field_ident);
    }

    fn add_for_copy_trait_check(&self, field_ctx: &FXFieldCtx) {
        self.copyable_types.borrow_mut().push(field_ctx.ty().clone());
    }

    #[cfg(feature = "serde")]
    fn add_shadow_field_decl(&self, field: TokenStream) {
        self.shadow_field_toks.borrow_mut().push(field);
    }

    #[cfg(feature = "serde")]
    fn add_shadow_default_decl(&self, field: TokenStream) {
        self.shadow_default_toks.borrow_mut().push(field);
    }

    fn methods_combined(&self) -> TokenStream {
        let method_toks = self.method_toks.borrow();
        quote! [ #( #method_toks )* ]
    }

    fn struct_fields(&self) -> Ref<Vec<TokenStream>> {
        self.field_toks.borrow()
    }

    fn builders_combined(&self) -> TokenStream {
        let builder_toks = &*self.builder_toks.borrow();
        quote! [
            #( #builder_toks )*
        ]
    }

    fn builder_fields_combined(&self) -> TokenStream {
        let build_field_toks = &*self.builder_field_toks.borrow();
        quote! [ #( #build_field_toks ),* ]
    }

    fn defaults_combined(&self) -> TokenStream {
        let default_toks = &*self.default_toks.borrow();
        quote! [ #( #default_toks ),* ]
    }

    // fn initializers_combined(&self) -> TokenStream {
    //     TokenStream::new()
    // }

    fn type_tokens<'s>(&'s self, field_ctx: &'s FXFieldCtx) -> &'s TokenStream {
        field_ctx.ty_wrapped(|| {
            // fxtrace!(field_ctx.ident_tok().to_string());
            let ty = field_ctx.ty();
            let span = ty.span();
            let rc = if field_ctx.is_lazy() {
                quote_spanned! [span=> ::fieldx::OnceCell<#ty>]
            }
            else if field_ctx.is_optional() {
                quote_spanned! [span=> ::std::option::Option<#ty>]
            }
            else {
                field_ctx.ty_tok().clone()
            };
            rc
        })
    }

    fn field_lazy_initializer(
        &self,
        fctx: &FXFieldCtx,
        self_ident: Option<TokenStream>,
    ) -> darling::Result<TokenStream> {
        let lazy_name = self.lazy_name(fctx)?;
        let ident = fctx.ident_tok();
        let self_var = self_ident.unwrap_or(quote![self]);
        Ok(quote![#self_var.#ident.get_or_init( || #self_var.#lazy_name() )])
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor() {
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let ident = fctx.ident()?;
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let ty = fctx.ty();
            let accessor_name = self.accessor_name(fctx)?;
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Accessor);

            let (reference, deref, meth) = match fctx.accessor_mode() {
                FXAccessorMode::Copy => (quote![], quote![*], quote![]),
                FXAccessorMode::Clone => (quote![], quote![], quote![.clone()]),
                FXAccessorMode::None => (quote![&], quote![], quote![]),
            };

            // fxtrace!();
            if fctx.is_lazy() {
                let lazy_init = self.field_lazy_initializer(fctx, None)?;

                Ok(quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> #reference #ty {
                        #deref #lazy_init #meth
                    }
                ])
            }
            else if fctx.is_optional() {
                let ty_tok = self.type_tokens(fctx);
                Ok(quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> #reference #ty_tok { #reference self.#ident #meth }
                ])
            }
            else {
                Ok(quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> #reference #ty { #reference self.#ident #meth }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor_mut() {
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::AccessorMut);
            let ty = fctx.ty();
            let accessor_name = self.accessor_mut_name(fctx)?;
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::AccessorMut);
            let span = self.helper_span(fctx, FXHelperKind::AccessorMut);

            if fctx.is_lazy() {
                let lazy_name = self.lazy_name(fctx)?;

                Ok(quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&mut self) -> &mut #ty {
                        self.#ident.get_or_init( || self.#lazy_name() );
                        self.#ident.get_mut().unwrap()
                    }
                ])
            }
            else if fctx.is_optional() {
                let ty_tok = self.type_tokens(fctx);
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #accessor_name(&mut self) -> &mut #ty_tok { &mut self.#ident }
                ])
            }
            else {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #accessor_name(&mut self) -> &mut #ty { &mut self.#ident }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, FXHelperKind::Builder);
        let field_ident = fctx.ident_tok();
        let field_default = self.field_default_wrap(fctx)?;

        Ok(if fctx.is_ignorable() || !fctx.needs_builder() {
            quote![]
        }
        else if fctx.is_lazy() {
            quote_spanned![span=>
                #field_ident: if self.#field_ident.is_some() {
                    ::fieldx::OnceCell::from(self.#field_ident.take().unwrap())
                }
                else {
                    #field_default
                }
            ]
        }
        else if fctx.is_optional() {
            quote_spanned![span=>
                #field_ident: if self.#field_ident.is_some() {
                    self.#field_ident.take()
                }
                else {
                    #field_default
                }
            ]
        }
        else {
            self.simple_field_build_setter(fctx, field_ident, &span)
        })
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_setter() {
            let setter_name = self.setter_name(fctx)?;
            let span = self.helper_span(fctx, FXHelperKind::Setter);
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Setter);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Setter);

            let (gen_params, val_type, into_tok) = self.into_toks(fctx, fctx.is_setter_into());

            if fctx.is_lazy() {
                Ok(quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #setter_name #gen_params(&mut self, value: #val_type) -> ::std::option::Option<#ty> {
                        let old = self.#ident.take();
                        let _ = self.#ident.set(value #into_tok);
                        old
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #setter_name #gen_params(&mut self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident.replace(value #into_tok)
                    }
                ])
            }
            else {
                Ok(quote_spanned![span=>
                    #attributes_fn
                    #vis_tok fn #setter_name #gen_params(&mut self, value: #val_type) -> #ty {
                        let old = self.#ident;
                        self.#ident = value #into_tok;
                        old
                    }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_clearer() {
            let clearer_name = self.clearer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Clearer);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Clearer);
            let span = self.helper_span(fctx, FXHelperKind::Clearer);

            Ok(quote_spanned! [span=>
                #attributes_fn
                #vis_tok fn #clearer_name(&mut self) -> ::std::option::Option<#ty> {
                    self.#ident.take()
                }
            ])
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_predicate() && (fctx.is_lazy() || fctx.is_optional()) {
            let predicate_name = self.predicate_name(fctx)?;
            let span = self.helper_span(fctx, FXHelperKind::Predicate);
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Predicate);
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Predicate);

            if fctx.is_lazy() {
                return Ok(quote_spanned! [span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #predicate_name(&self) -> bool {
                        self.#ident.get().is_some()
                    }
                ]);
            }
            else if fctx.is_optional() {
                return Ok(quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #predicate_name(&self) -> bool {
                        self.#ident.is_some()
                    }
                ]);
            }
        }

        Ok(TokenStream::new())
    }

    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: Option<TokenStream>) -> darling::Result<TokenStream> {
        Ok(if fctx.is_lazy() {
            value.map_or_else(
                || quote![::fieldx::OnceCell::new()],
                |value| quote! [ ::fieldx::OnceCell::from(#value) ],
            )
        }
        else if fctx.is_optional() {
            value.map_or_else(
                || quote![::std::option::Option::None],
                |value| quote! [ ::std::option::Option::Some(#value) ],
            )
        }
        else {
            value.map(|value| quote! [ #value ]).ok_or_else(|| {
                darling::Error::custom(format!("No value was supplied for plain field {}", fctx.ident_str()))
            })?
        })
    }

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.field_value_wrap(fctx, self.field_default_value(fctx)?.map(|dv| self.fixup_self_type(dv)))
    }

    // Reader/writer make no sense for non-sync. Hence do nothing.
    fn field_reader(&self, _fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(quote![])
    }

    fn field_writer(&self, _fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(quote![])
    }

    #[cfg(feature = "serde")]
    fn field_from_shadow(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let shadow_var = self.ctx().shadow_var_ident();

        Ok(if self.is_serde_optional(fctx) {
            let default_value = self.field_default_wrap(fctx)?;
            let shadow_value = self.field_value_wrap(fctx, Some(quote![v]))?;
            quote![ #shadow_var.#field_ident.map_or_else(|| #default_value, |v| #shadow_value) ]
        }
        else {
            quote![ #shadow_var.#field_ident ]
        })
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let me_var = self.ctx().me_var_ident();

        Ok(if fctx.is_lazy() {
            quote![ #me_var.#field_ident.take() ]
        }
        else if fctx.is_optional() {
            quote![ #me_var.#field_ident ]
        }
        else {
            quote![ #me_var.#field_ident ]
        })
    }

    fn struct_extras(&self) {
        let ctx = self.ctx();
        let generics = ctx.input().generics();
        let generic_params = self.generic_params();
        let input = ctx.input_ident();
        let where_clause = &generics.where_clause;
        ctx.tokens_extend(quote![
            impl #generics ::fieldx::traits::FXStructNonSync for #input #generic_params #where_clause {}
        ]);

        self.add_method_decl(quote![
            #[inline]
            fn __fieldx_new() -> Self {
                Self::default()
            }
        ]);

        if ctx.args().needs_new() {
            self.add_method_decl(quote![
                #[inline]
                pub fn new() -> Self {
                    Self::default()
                }
            ])
        }
    }
}
