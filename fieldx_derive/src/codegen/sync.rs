use crate::codegen::{
    context::{FXCodeGenCtx, FXFieldCtx},
    FXCGen,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::cell::RefCell;
use syn::spanned;

pub struct FXCodeGen {
    ctx:              FXCodeGenCtx,
    field_toks:       RefCell<Vec<TokenStream>>,
    default_toks:     RefCell<Vec<TokenStream>>,
    method_toks:      RefCell<Vec<TokenStream>>,
    initializer_toks: RefCell<Vec<TokenStream>>,
}

impl FXCodeGen {
    pub fn new(ctx: FXCodeGenCtx) -> Self {
        Self {
            ctx,
            field_toks: RefCell::new(vec![]),
            default_toks: RefCell::new(vec![]),
            method_toks: RefCell::new(vec![]),
            initializer_toks: RefCell::new(vec![]),
        }
    }
}

impl FXCodeGen {
    // This variable will hold Arc::new(self) value for __fieldx_init method implementation.
    fn arc_self(&self) -> TokenStream {
        quote![arc_self]
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
        self.initializer_toks.borrow_mut().push(initializer)
    }

    fn add_method_decl(&self, method: TokenStream) {
        self.method_toks.borrow_mut().push(method);
    }

    fn methods_combined(&self) -> TokenStream {
        let method_toks = self.method_toks.borrow();
        quote! [ #( #method_toks )* ]
    }

    fn fields_combined(&self) -> TokenStream {
        let field_toks = self.field_toks.borrow();
        quote! [ #( #field_toks ),* ]
    }

    fn defaults_combined(&self) -> TokenStream {
        let default_toks = self.default_toks.borrow();
        quote! [ #( #default_toks ),* ]
    }

    fn initializers_combined(&self) -> TokenStream {
        let initializer_toks = self.initializer_toks.borrow();
        quote![ #( #initializer_toks )* ]
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> &'s TokenStream {
        fctx.ty_wrapped(|| {
            let ty_tok = fctx.ty_tok();
            let span = (fctx.ty() as &dyn spanned::Spanned).span();

            if fctx.is_lazy() {
                quote_spanned! [span=> fieldx::FXProxy<#ty_tok>]
            }
            else if fctx.is_optional() {
                quote_spanned! [span=> fieldx::RwLock<Option<#ty_tok>>]
            }
            else {
                ty_tok.clone()
            }
        })
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor(true) {
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();
            let accessor_name = if fctx.accessor().is_some() {
                self.helper_name_tok(fctx, fctx.accessor(), None, "accessor")?
            }
            else {
                quote![#ident]
            };

            if fctx.is_lazy() || fctx.is_optional() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #accessor_name(&self) -> Option<#ty> {
                        *self.#ident.read()
                    }
                ])
            }
            else {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #accessor_name(&self) -> &#ty {
                        &self.#ident
                    }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_reader() && (fctx.is_lazy() || fctx.is_optional()) {
            let reader_name = self.helper_name_tok(fctx, fctx.reader(), Some("read"), "reader")?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            Ok(quote_spanned! [*fctx.span()=>
                #[inline]
                #pub_tok fn #reader_name(&self) -> fieldx::RwLockReadGuard<Option<#ty>> {
                    self.#ident.read()
                }
            ])
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_writer() && (fctx.is_lazy() || fctx.is_optional()) {
            let writer_name = self.helper_name_tok(fctx, fctx.writer(), Some("write"), "writer")?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #writer_name<'a>(&'a self) -> fieldx::FXWrLock<'a, #ty> {
                        self.#ident.write()
                    }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #writer_name(&self) -> fieldx::RwLockWriteGuard<Option<#ty>> {
                        self.#ident.write()
                    }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_setter() {
            let set_name = self.helper_name_tok(fctx, fctx.setter(), Some("set"), "setter")?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #set_name(&self, value: #ty) -> Option<#ty> {
                        self.#ident.write().store(value)
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #set_name(&self, value: #ty) -> Option<#ty> {
                        self.#ident.write().replace(value)
                    }
                ])
            }
            else {
                Ok(quote_spanned! [*fctx.span()=>
                    #pub_tok fn #set_name(&mut self, value: #ty) -> #ty {
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

    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_clearer() {
            let clear_name = self.helper_name_tok(fctx, fctx.clearer(), Some("clear"), "clearer")?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned![*fctx.span()=>
                   #[inline]
                   #pub_tok fn #clear_name(&self) -> Option<#ty> {
                       self.#ident.clear()
                   }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #clear_name(&self) -> Option<#ty> {
                       self.#ident.write().take()
                    }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_predicate() {
            let pred_name = self.helper_name_tok(fctx, fctx.predicate(), Some("has"), "predicate")?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();

            if fctx.is_lazy() {
                Ok(quote_spanned![*fctx.span()=>
                   #[inline]
                   #pub_tok fn #pred_name(&self) -> bool {
                      self.#ident.is_set()
                   }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #pred_name(&self) -> bool {
                      self.#ident.read().is_some()
                   }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let def_tok = self.field_default_value(fctx)?;
        if fctx.is_optional() && !fctx.is_lazy() {
            Ok(quote! [ fieldx::RwLock::new(Some(#def_tok)) ])
        }
        else {
            Ok(quote! [#def_tok])
        }
    }

    fn field_initializer(&self, fctx: &FXFieldCtx) {
        if fctx.is_lazy() {
            let ident = fctx.ident_tok();
            let builder_name = match self.helper_name_tok(fctx, fctx.lazy(), Some("build"), "builder") {
                Ok(name) => name,
                Err(err) => {
                    self.ctx.push_error(err);
                    return;
                }
            };
            let arc_self = self.arc_self();

            self.add_initializer_decl(quote! [
                let self_weak = fieldx::Arc::downgrade(&#arc_self);
                let callback = Box::new(move || self_weak.upgrade().unwrap().#builder_name());
                #arc_self.#ident.proxy_setup(callback);
            ]);
        }
    }

    fn struct_extras(&self) {
        let arc_self = self.arc_self();
        let initializers = self.initializer_toks.borrow_mut();

        self.add_method_decl(quote! [
            fn __fieldx_init(self) -> fieldx::Arc<Self> {
                let #arc_self = fieldx::Arc::new(self);
                #( #initializers )*
                #arc_self
            }
        ]);

        if self.ctx.needs_new {
            self.add_method_decl(quote![
                #[inline]
                pub fn new() -> fieldx::Arc<Self> {
                    Self::default().__fieldx_init()
                }
            ])
        }
    }
}
