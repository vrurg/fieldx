use crate::codegen::{
    context::{FXCodeGenCtx, FXFieldCtx},
    DError, DResult, FXCGen,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::cell::RefCell;

pub struct FXCodeGen {
    ctx:          FXCodeGenCtx,
    field_toks:   RefCell<Vec<TokenStream>>,
    default_toks: RefCell<Vec<TokenStream>>,
    method_toks:  RefCell<Vec<TokenStream>>,
}

impl FXCodeGen {
    pub fn new(ctx: FXCodeGenCtx) -> Self {
        Self {
            ctx,
            field_toks: RefCell::new(vec![]),
            default_toks: RefCell::new(vec![]),
            method_toks: RefCell::new(vec![]),
        }
    }
}

impl FXCGen for FXCodeGen {
    fn ctx(&self) -> &FXCodeGenCtx {
        &self.ctx
    }

    fn add_field_decl(&self, field: TokenStream) {
        self.field_toks.borrow_mut().push(field);
    }

    fn add_defaults_decl(&self, defaults: TokenStream) {
        self.default_toks.borrow_mut().push(defaults);
    }

    fn add_initializer_decl(&self, initializer: TokenStream) {
        let ctx = self.ctx();
        let ident = ctx.input_ident();
        ctx.push_error(
            DError::custom(format!("Can't add an initializer to a non-sync struct {}", ident))
                .with_span(&initializer)
                .note("This is an internal error, a bug in fxstruct implementation is assumed"),
        )
    }

    fn add_method_decl(&self, method: TokenStream) {
        self.method_toks.borrow_mut().push(method);
    }

    fn methods_combined(&self) -> TokenStream {
        let method_toks = self.method_toks.borrow();
        quote! [ #( #method_toks )* ]
    }

    fn fields_combined(&self) -> TokenStream {
        let field_toks = &*self.field_toks.borrow();
        quote! [ #( #field_toks ),* ]
    }

    fn defaults_combined(&self) -> TokenStream {
        let default_toks = &*self.default_toks.borrow();
        quote! [ #( #default_toks ),* ]
    }

    fn initializers_combined(&self) -> TokenStream {
        TokenStream::new()
    }

    fn type_tokens<'s>(&'s self, field_ctx: &'s FXFieldCtx) -> &'s TokenStream {
        let field = field_ctx.field();
        field_ctx.ty_wrapped(|| {
            let ty = field_ctx.ty();
            if field.is_lazy() {
                quote! [fieldx::OnceCell<#ty>]
            }
            else if field.is_optional() {
                quote! [Option<#ty>]
            }
            else {
                field_ctx.ty_tok().clone()
            }
        })
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if fctx.needs_accessor(false) {
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();
            let accessor_name = self.accessor_name(fctx)?;

            if fctx.is_lazy() {
                let builder_name = self.builder_name(fctx)?;

                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&self) -> &#ty { self.#ident.get_or_init( move || self.#builder_name() ) }
                ])
            }
            else if fctx.is_optional() {
                let ty_tok = self.type_tokens(fctx);
                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&self) -> &#ty_tok { &self.#ident }
                ])
            }
            else {
                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&self) -> &#ty { &self.#ident }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if fctx.needs_accessor_mut() {
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();
            let accessor_name = self.accessor_mut_name(fctx)?;

            if fctx.is_lazy() {
                let builder_name = self.builder_name(fctx)?;

                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&mut self) -> &mut #ty { self.#ident.get_or_init( || self.#builder_name() ); self.#ident.get_mut().unwrap() }
                ])
            }
            else if fctx.is_optional() {
                let ty_tok = self.type_tokens(fctx);
                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&mut self) -> &mut #ty_tok { &mut self.#ident }
                ])
            }
            else {
                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&mut self) -> &mut #ty { &mut self.#ident }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if fctx.needs_setter() {
            let setter_name = self.setter_name(fctx)?;
            let span = *fctx.span();
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned![span=>
                        #pub_tok fn #setter_name(&mut self, value: #ty) -> Option<#ty> {
                            let old = self.#ident.take();
                            let _ = self.#ident.set(value);
                            old
                        }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned! [span=>
                    #pub_tok fn #setter_name(&mut self, value: #ty) -> Option<#ty> { self.#ident.replace(value) } ])
            }
            else {
                Ok(quote_spanned![span=>
                        #pub_tok fn #setter_name(&mut self, value: #ty) -> #ty {
                            let old = self.#ident;
                            self.#ident = value;
                            old
                        }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_clearer(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if fctx.needs_clearer() {
            let clearer_name = self.clearer_name(fctx)?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();
            Ok(
                quote_spanned! [*fctx.span()=> #pub_tok fn #clearer_name(&mut self) -> Option<#ty> { self.#ident.take() } ],
            )
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        if fctx.needs_predicate() && (fctx.is_lazy() || fctx.is_optional()) {
            let predicate_name = self.predicate_name(fctx)?;
            let span = *fctx.span();
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();

            if fctx.is_lazy() {
                return Ok(
                    quote_spanned! [span=> #pub_tok fn #predicate_name(&self) -> bool { self.#ident.get().is_some() } ],
                );
            }
            else if fctx.is_optional() {
                return Ok(
                    quote_spanned! [span=> #pub_tok fn #predicate_name(&self) -> bool { self.#ident.is_some() } ],
                );
            }
        }

        Ok(TokenStream::new())
    }

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        let def_tok = self.field_default_value(fctx)?;

        if fctx.is_lazy() {
            Ok(quote! [ fieldx::OnceCell::new() ])
        }
        else if fctx.is_optional() {
            if fctx.has_default() {
                Ok(quote! [ Some(#def_tok) ])
            }
            else {
                Ok(quote! [ None ])
            }
        }
        else {
            Ok(quote! [ #def_tok ])
        }
    }

    // Reader/writer make no sense for non-sync. Hence do nothing.
    fn field_reader(&self, _fctx: &FXFieldCtx) -> DResult<TokenStream> {
        Ok(TokenStream::new())
    }

    fn field_writer(&self, _fctx: &FXFieldCtx) -> DResult<TokenStream> {
        Ok(TokenStream::new())
    }

    // No-op since initialization is required by sync version only
    fn field_initializer(&self, _fctx: &FXFieldCtx) {}

    fn struct_extras(&self) {
        if self.ctx.needs_new {
            self.add_method_decl(quote![
                #[inline]
                pub fn new() -> Self {
                    Self::default()
                }
            ])
        }
    }
}
