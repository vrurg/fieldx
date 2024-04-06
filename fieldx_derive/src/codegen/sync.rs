use crate::codegen::{
    context::{FXCodeGenCtx, FXFieldCtx},
    DResult, FXCGen,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::cell::RefCell;
use syn::spanned::Spanned;

pub(crate) struct FXCodeGen<'f> {
    ctx:                FXCodeGenCtx,
    field_toks:         RefCell<Vec<TokenStream>>,
    default_toks:       RefCell<Vec<TokenStream>>,
    method_toks:        RefCell<Vec<TokenStream>>,
    initializer_toks:   RefCell<Vec<TokenStream>>,
    builder_field_toks: RefCell<Vec<TokenStream>>,
    builder_toks:       RefCell<Vec<TokenStream>>,
    builder_field_ctx:  RefCell<Vec<FXFieldCtx<'f>>>,
    copyable_types:     RefCell<Vec<syn::Type>>,
}

impl<'f> FXCodeGen<'f> {
    pub fn new(ctx: FXCodeGenCtx) -> Self {
        Self {
            ctx,
            field_toks: RefCell::new(vec![]),
            default_toks: RefCell::new(vec![]),
            method_toks: RefCell::new(vec![]),
            initializer_toks: RefCell::new(vec![]),
            builder_field_toks: RefCell::new(vec![]),
            builder_field_ctx: RefCell::new(vec![]),
            builder_toks: RefCell::new(vec![]),
            copyable_types: RefCell::new(vec![]),
        }
    }
}

impl<'f> FXCodeGen<'f> {
    // This variable will hold Arc::new(self) value for __fieldx_init method implementation.
    fn arc_self(&self) -> TokenStream {
        quote![arc_self]
    }

    fn field_initializer_toks(&self, fctx: &FXFieldCtx) -> DResult<TokenStream> {
        Ok(if fctx.is_lazy() {
            let ident = fctx.ident_tok();
            let lazy_name = self.lazy_name(fctx)?;
            let arc_self = self.arc_self();

            quote! [
                let self_weak = ::fieldx::Arc::downgrade(&#arc_self);
                let callback = ::std::boxed::Box::new(move || self_weak.upgrade().unwrap().#lazy_name());
                #arc_self.#ident.proxy_setup(callback);
            ]
        }
        else {
            quote![]
        })
    }
}

impl<'f> FXCGen<'f> for FXCodeGen<'f> {
    fn ctx(&self) -> &FXCodeGenCtx {
        &self.ctx
    }

    fn copyable_types(&self) -> std::cell::Ref<Vec<syn::Type>> {
        self.copyable_types.borrow()
    }

    fn add_field_decl(&self, field: TokenStream) {
        self.field_toks.borrow_mut().push(field);
    }

    fn add_defaults_decl(&self, defaults: TokenStream) {
        self.default_toks.borrow_mut().push(defaults);
    }

    fn add_initializer_decl(&self, initializer: TokenStream) {
        if !initializer.is_empty() {
            self.initializer_toks.borrow_mut().push(initializer)
        }
    }

    fn add_method_decl(&self, method: TokenStream) {
        if !method.is_empty() {
            self.method_toks.borrow_mut().push(method);
        }
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

    fn add_builder_field_ctx(&self, fctx: FXFieldCtx<'f>) {
        self.builder_field_ctx.borrow_mut().push(fctx);
    }

    fn check_for_impl_copy(&self, field_ctx: &FXFieldCtx) {
        self.copyable_types.borrow_mut().push(field_ctx.ty().clone());
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

    fn builders_combined(&self) -> TokenStream {
        let builder_toks = &*self.builder_toks.borrow();
        quote![
            #( #builder_toks )*
        ]
    }

    fn builder_fields_combined(&self) -> TokenStream {
        let build_field_toks = &*self.builder_field_toks.borrow();
        quote! [ #( #build_field_toks ),* ]
    }

    fn builder_fields_ctx(&self) -> std::cell::Ref<Vec<FXFieldCtx<'f>>> {
        self.builder_field_ctx.borrow()
    }

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> &'s TokenStream {
        fctx.ty_wrapped(|| {
            let ty_tok = fctx.ty_tok();
            let span = fctx.ty().span();

            if fctx.is_lazy() {
                quote_spanned! [span=> ::fieldx::FXProxy<#ty_tok>]
            }
            else if fctx.is_optional() {
                quote_spanned! [span=> ::fieldx::RwLock<Option<#ty_tok>>]
            }
            else {
                ty_tok.clone()
            }
        })
    }

    // fn type_tokens_mut<'s>(&'s self,field_ctx: &'s FXFieldCtx) ->  &'s TokenStream {
    //     self.type_tokens(field_ctx)
    // }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor(true) {
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let accessor_name = self.accessor_name(fctx)?;
            let ty = fctx.ty();
            let is_optional = fctx.is_optional();

            if fctx.is_lazy() || is_optional {
                let cmethod = if fctx.is_copy() {
                    if is_optional {
                        quote![.as_ref().unwrap().as_ref().copied()]
                    }
                    else {
                        quote![]
                    }
                }
                else {
                    if is_optional {
                        quote![.as_ref().unwrap().as_ref().cloned()]
                    }
                    else {
                        quote![.clone()]
                    }
                };

                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&self) -> #ty {
                        let rlock = self.#ident.read();
                        (*rlock)#cmethod
                    }
                ])
            }
            else {
                let cmethod = if fctx.is_copy() { quote![] } else { quote![.clone()] };

                Ok(quote_spanned![*fctx.span()=>
                    #pub_tok fn #accessor_name(&self) -> #ty {
                        self.#ident #cmethod
                    }
                ])
            }
        }
        else {
            Ok(quote![])
        }
    }

    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor_mut() {
            Err(
                darling::Error::custom("Mutable accessors are not supported for sync structs")
                    .with_span(&fctx.accessor_mut().span()),
            )
        }
        else {
            Ok(quote![])
        }
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = fctx.span();
        let field_ident = fctx.ident_tok();
        let field_default = self.ok_or(self.field_default_wrap(fctx));
        let field_type = self.type_tokens(fctx);

        Ok(if fctx.is_ignorable() || !self.field_needs_builder(fctx) {
            quote![]
        }
        else if fctx.is_lazy() {
            quote_spanned![*span=>
                #field_ident: if self.#field_ident.is_some() {
                    // If builder has a value for the field then wrap the value into FXProxy using From/Into
                    let new: #field_type = self.#field_ident.take().unwrap().into();
                    new
                }
                else {
                    #field_default
                }
            ]
        }
        else if fctx.is_optional() {
            quote_spanned![*span=>
                // Optional can simply pickup and own either Some() or None from the builder.
                #field_ident: ::fieldx::RwLock::new(self.#field_ident.take())
            ]
        }
        else {
            self.simple_field_build_setter(fctx, field_ident, span)
        })
    }

    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_reader() && (fctx.is_lazy() || fctx.is_optional()) {
            let reader_name = self.reader_name(fctx)?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #reader_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::MappedRwLockReadGuard<'fx_reader_lifetime, #ty> {
                        self.#ident.read()
                    }
                ])
            }
            else {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #reader_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::MappedRwLockReadGuard<'fx_reader_lifetime, #ty> {
                        ::fieldx::RwLockReadGuard::map(self.#ident.read(), |data: &Option<#ty>| data.as_ref().unwrap())
                    }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_writer() && (fctx.is_lazy() || fctx.is_optional()) {
            let writer_name = self.writer_name(fctx)?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #writer_name<'fx_writer_lifetime>(&'fx_writer_lifetime self) -> ::fieldx::FXWrLock<'fx_writer_lifetime, #ty> {
                        self.#ident.write()
                    }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #writer_name(&self) -> ::fieldx::RwLockWriteGuard<::std::option::Option<#ty>> {
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
            let set_name = self.setter_name(fctx)?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #pub_tok fn #set_name(&self, value: #ty) -> ::std::option::Option<#ty> {
                        self.#ident.write().store(value)
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #set_name(&self, value: #ty) -> ::std::option::Option<#ty> {
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
            let clear_name = self.clearer_name(fctx)?;
            let ident = fctx.ident_tok();
            let pub_tok = fctx.pub_tok();
            let ty = fctx.ty();

            if fctx.is_lazy() {
                Ok(quote_spanned![*fctx.span()=>
                   #[inline]
                   #pub_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
                       self.#ident.clear()
                   }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #pub_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
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
            let pred_name = self.predicate_name(fctx)?;
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
        if fctx.is_lazy() {
            let ty_tok = fctx.ty_tok();
            if def_tok.is_empty() {
                Ok(quote![::std::default::Default::default()])
            }
            else {
                Ok(quote![ ::fieldx::FXProxy::<#ty_tok>::from(#def_tok) ])
            }
        }
        else if fctx.is_optional() {
            Ok(quote![ ::fieldx::RwLock::new(Some(#def_tok)) ])
        }
        else {
            Ok(quote![ #def_tok ])
        }
    }

    fn field_initializer(&self, fctx: &FXFieldCtx) {
        self.add_initializer_decl(self.ok_or(self.field_initializer_toks(fctx)))
    }

    fn wrap_construction(&self, construction: TokenStream) -> TokenStream {
        quote![
            (#construction).__fieldx_init()
        ]
    }

    fn builder_return_type(&self) -> TokenStream {
        let builder_ident = self.ctx().input_ident();
        let generic_params = self.generic_params();
        quote![::fieldx::Arc<#builder_ident #generic_params>]
    }

    fn builder_trait(&self) -> TokenStream {
        quote![::fieldx::traits::FXStructBuilderSync]
    }

    fn struct_extras(&self) {
        let arc_self = self.arc_self();
        let initializers = self.initializer_toks.borrow_mut();
        let ctx = self.ctx();
        let generics = ctx.input().generics();
        let generic_params = self.generic_params();
        let input = ctx.input_ident();
        let where_clause = &generics.where_clause;

        ctx.tokens_extend(quote![
            impl #generics ::fieldx::traits::FXStructSync for #input #generic_params
            #where_clause
            {
                fn __fieldx_init(self) -> ::fieldx::Arc<Self> {
                    let #arc_self = ::fieldx::Arc::new(self);
                    #( #initializers )*
                    #arc_self
                }

                #[inline]
                fn __fieldx_new() -> ::fieldx::Arc<Self> {
                    Self::default().__fieldx_init()
                }
            }
        ]);

        if ctx.needs_new() {
            self.add_method_decl(quote![
                pub fn new() -> ::fieldx::Arc<Self> {
                    Self::default().__fieldx_init()
                }
            ])
        }
    }
}
