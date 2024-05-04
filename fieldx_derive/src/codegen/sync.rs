use crate::{
    codegen::{
        context::{FXCodeGenCtx, FXFieldCtx, FXGenStage},
        DResult, FXCGen, FXOrig,
    },
    // util::TODO,
};
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    iter::Iterator,
};
use syn::spanned::Spanned;

use super::FXHelperKind;

pub(crate) struct FXCodeGen<'f> {
    ctx:                 FXCodeGenCtx,
    field_ctx_table:     RefCell<HashMap<syn::Ident, FXFieldCtx<'f>>>,
    field_toks:          RefCell<Vec<TokenStream>>,
    shadow_field_toks:   RefCell<Vec<TokenStream>>,
    default_toks:        RefCell<Vec<TokenStream>>,
    shadow_default_toks: RefCell<Vec<TokenStream>>,
    method_toks:         RefCell<Vec<TokenStream>>,
    initializer_toks:    RefCell<Vec<TokenStream>>,
    builder_field_toks:  RefCell<Vec<TokenStream>>,
    builder_toks:        RefCell<Vec<TokenStream>>,
    builder_field_ident: RefCell<Vec<syn::Ident>>,
    copyable_types:      RefCell<Vec<syn::Type>>,
}

impl<'f> FXCodeGen<'f> {
    pub fn new(ctx: FXCodeGenCtx) -> Self {
        Self {
            ctx,
            field_ctx_table: RefCell::new(HashMap::new()),
            field_toks: RefCell::new(vec![]),
            shadow_field_toks: RefCell::new(vec![]),
            default_toks: RefCell::new(vec![]),
            shadow_default_toks: RefCell::new(vec![]),
            method_toks: RefCell::new(vec![]),
            initializer_toks: RefCell::new(vec![]),
            builder_field_toks: RefCell::new(vec![]),
            builder_field_ident: RefCell::new(vec![]),
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

    #[cfg(not(feature = "serde"))]
    #[inline]
    fn filed_proxy_type(&self, _fctx: &FXFieldCtx) -> TokenStream {
        quote![FXProxy]
    }
}

// Methods related to `serde` feature
#[cfg(feature = "serde")]
impl<'f> FXCodeGen<'f> {
    fn add_shadow_field_decl(&self, field: TokenStream) {
        self.shadow_field_toks.borrow_mut().push(field);
    }

    fn add_shadow_default_decl(&self, field: TokenStream) {
        self.shadow_default_toks.borrow_mut().push(field);
    }

    // Field is an Option in the shadow struct if it is optional or lazy and has no default value
    pub fn is_serde_optional(&self, fctx: &FXFieldCtx) -> bool {
        (fctx.is_optional() || fctx.is_lazy()) && !fctx.has_default_value()
    }

    fn filter_shadow_attributes<'a>(&'a self, fctx: &'a FXFieldCtx) -> impl Iterator<Item = &'a syn::Attribute> {
        // Only use `serde` attribute and those listed in forward_attrs
        let serde_helper = fctx.serde().as_ref();
        fctx.attrs()
            .iter()
            .filter(move |a| a.path().is_ident("serde") || serde_helper.map_or(false, |sh| sh.accepts_attr(a)))
    }

    fn shadow_field(&self, fctx: &FXFieldCtx) {
        let ident = fctx.ident_tok();
        let mut ty = fctx.ty_tok().clone();
        let vis = fctx.vis();
        let attrs = self.filter_shadow_attributes(fctx);
        let serde_attr = self.serde_attribute(fctx);

        if self.is_serde_optional(fctx) {
            ty = quote![ ::std::option::Option<#ty> ];
        }

        self.add_shadow_field_decl(quote_spanned! [*fctx.span()=>
            #( #attrs )*
            #serde_attr
            #vis #ident: #ty
        ]);
    }

    fn shadow_field_default(&self, fctx: &FXFieldCtx) {
        let mut default_tok = self.fixup_self_for_shadow(self.ok_or(self.field_default_value(fctx)));
        let field_ident = fctx.ident_tok();

        if default_tok.is_empty() {
            default_tok = quote![::std::default::Default::default()];
        }

        self.add_shadow_default_decl(quote![ #field_ident: #default_tok ]);
    }

    fn shadow_struct(&self) {
        let ctx = self.ctx();
        let _state_guard = ctx.push_state(FXGenStage::ShadowStruct);
        let args = ctx.args();
        if args.is_serde() {
            let serde = args.serde().as_ref().unwrap();
            let shadow_ident = ctx.shadow_ident();
            let fields = self.shadow_field_toks.borrow();
            let mut attrs = vec![];
            let derive_attr = self.derive_toks(&self.serde_derive_traits());
            let mut default_impl = quote![];

            attrs.push(derive_attr);

            if serde.has_default() {
                let default_value = serde.default_value().as_ref().unwrap();
                let orig_span = default_value.orig().map_or_else(|| Span::call_site(), |s| s.span());

                if let Some(serde_default_str) = default_value.value() {
                    attrs.push(quote_spanned! [orig_span=>
                        #[serde(default = #serde_default_str)]
                    ]);
                }
                else {
                    attrs.push(quote_spanned! [orig_span=>
                        #[serde(default)]
                    ]);
                }

                let default_toks = self.shadow_default_toks.borrow();
                default_impl = quote![
                    impl Default for #shadow_ident {
                        fn default() -> Self {
                            Self {
                                #( #default_toks ),*
                            }
                        }
                    }
                ];
            }

            ctx.tokens_extend(quote![
                #( #attrs )*
                struct #shadow_ident {
                    #( #fields ),*
                }

                #default_impl
            ]);
        }
    }

    // Impl From for the shadow struct
    fn struct_from_shadow(&'f self) {
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_serde() {
            let shadow_ident = ctx.shadow_ident();
            let struct_ident = ctx.input_ident();
            let mut fields = vec![];

            for field in ctx.input().fields() {
                let fctx = self.field_ctx(field.ident().as_ref().unwrap());
                if let Ok(fctx) = fctx {
                    if fctx.is_serde() {
                        let field_ident = fctx.ident_tok();

                        let fetch_shadow_field = if self.is_serde_optional(&*fctx) {
                            quote![
                                // Try initializating struct's field with default if shadow field is None
                                shadow.#field_ident.map_or_else(|| ::std::default::Default::default(), |v| v.into())
                            ]
                        }
                        else {
                            quote! [
                                shadow.#field_ident.into()
                            ]
                        };

                        fields.push(quote![
                            #field_ident: #fetch_shadow_field
                        ]);
                    }
                }
                else {
                    self.ctx().push_error(fctx.unwrap_err())
                }
            }

            ctx.tokens_extend(quote![
                impl<'de> ::serde::Deserialize<'de> for #struct_ident {
                    fn deserialize<__D>(deserializer: __D) -> ::std::result::Result<Self, __D::Error>
                    where
                        __D: ::serde::Deserializer<'de>,
                    {
                        let shadow = <#shadow_ident as ::serde::Deserialize>::deserialize(deserializer)?;
                        #struct_ident {
                            #( #fields, )*
                            .. Self::default()
                        }
                    }
                }
            ]);

            ctx.tokens_extend(quote![
                impl ::std::convert::From<#shadow_ident> for ::std::sync::Arc<#struct_ident> {
                    fn from(shadow: #shadow_ident) -> Self {
                        let me = #struct_ident {
                            #( #fields, )*
                            .. Self::default()
                        };
                        me.__fieldx_init()
                    }
                }
            ]);
        }
    }

    fn fixup_self_for_shadow(&self, tokens: TokenStream) -> TokenStream {
        let mut fixed_tokens = quote![];
        let struct_ident = self.ctx().input_ident().to_string();
        fixed_tokens.extend(tokens.clone().into_iter().map(|t| match t {
            TokenTree::Ident(ref ident) => {
                if ident.to_string() == "Self" {
                    TokenTree::Ident(proc_macro2::Ident::new(&struct_ident, ident.span()))
                }
                else {
                    t
                }
            }
            TokenTree::Group(ref group) => TokenTree::Group(proc_macro2::Group::new(
                group.delimiter(),
                self.fixup_self_for_shadow(group.stream()),
            )),
            _ => t,
        }));
        fixed_tokens
    }

    fn field_proxy_type(&self, fctx: &FXFieldCtx) -> TokenStream {
        if fctx.is_serde() {
            quote![FXProxySerde]
        }
        else {
            quote![FXProxy]
        }
    }
}

impl<'f> FXCGen<'f> for FXCodeGen<'f> {
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

    fn add_builder_field_ident(&self, field_ident: syn::Ident) {
        self.builder_field_ident.borrow_mut().push(field_ident);
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

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> &'s TokenStream {
        fctx.ty_wrapped(|| {
            let ty_tok = fctx.ty_tok();
            let span = fctx.ty().span();

            if fctx.is_lazy() {
                let proxy_type = self.field_proxy_type(fctx);
                quote_spanned! [span=> ::fieldx::#proxy_type<#ty_tok>]
            }
            else if fctx.is_optional() {
                quote_spanned! [span=> ::fieldx::RwLock<Option<#ty_tok>>]
            }
            else if fctx.needs_reader() || fctx.needs_writer() {
                quote_spanned! [span=> ::fieldx::RwLock<#ty_tok>]
            }
            else {
                ty_tok.clone()
            }
        })
    }

    #[cfg(feature = "serde")]
    fn serde_attribute(&self, fctx: &FXFieldCtx) -> TokenStream {
        if self.ctx().args().is_serde() {
            let skip_toks = self.field_serde_skip_toks(fctx);
            let mut serde_attr_args = vec![];

            if !skip_toks.is_empty() {
                serde_attr_args.push(skip_toks);
            }

            let mut default_arg = None;

            if let Some(serde_helper) = fctx.serde().as_ref() {
                if let Some(default_value) = serde_helper.default_value() {
                    if let Some(default_value) = default_value.value() {
                        default_arg = Some(quote![default = #default_value]);
                    }
                    else {
                        default_arg = Some(quote![default]);
                    }
                }
            }

            if default_arg.is_none() && (fctx.has_default_value() || fctx.is_optional() || fctx.is_lazy()) {
                default_arg = Some(quote![default])
            }

            if let Some(default_arg) = default_arg {
                serde_attr_args.push(default_arg);
            }

            if serde_attr_args.is_empty() {
                quote![]
            }
            else {
                quote! [ #[serde( #( #serde_attr_args ),* )] ]
            }
        }
        else {
            quote![]
        }
    }

    #[cfg(feature = "serde")]
    fn serde_struct_attribute(&self) -> TokenStream {
        let ctx = self.ctx();
        let args = ctx.args();

        if args.is_serde() {
            let mut serde_args: Vec<TokenStream> = vec![];

            let serde_helper = args.serde().as_ref().unwrap();
            let shadow_ident = ctx.shadow_ident().to_string();

            if serde_helper.needs_deserialize().unwrap_or(true) {
                // serde_args.push(quote![from = #shadow_ident]);
            }
            if serde_helper.needs_serialize().unwrap_or(true) {
                // serde_args.push(quote![into = #shadow_ident]);
            }

            if serde_args.len() > 0 {
                quote![ #[serde( #( #serde_args ),*  )] ]
            }
            else {
                quote![]
            }
        }
        else {
            quote![]
        }
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor() {
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let accessor_name = self.accessor_name(fctx)?;
            let is_optional = fctx.is_optional();
            let ty = fctx.ty();
            let is_copy = fctx.is_copy();
            let attributes_fn = fctx.attributes_fn(fctx.accessor().as_ref());

            if fctx.is_lazy() || fctx.needs_lock() || is_optional {
                let ty = if is_optional { quote![Option<#ty>] } else { quote![#ty] };

                let cmethod = if is_copy {
                    if is_optional {
                        quote![.as_ref().copied()]
                    }
                    else {
                        quote![]
                    }
                }
                else {
                    if is_optional {
                        quote![.as_ref().cloned()]
                    }
                    else {
                        quote![.clone()]
                    }
                };

                Ok(quote_spanned![*fctx.span()=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> #ty {
                        let rlock = self.#ident.read();
                        (*rlock)#cmethod
                    }
                ])
            }
            else {
                let cmethod = if fctx.is_copy() { quote![] } else { quote![.clone()] };

                Ok(quote_spanned![*fctx.span()=>
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> #ty {
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

        Ok(if fctx.is_ignorable() || !fctx.needs_builder() {
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
        // else if fctx.needs_lock() {
        //     quote_spanned![*span=>
        //         // When lock is needed then we need to wrap the value in RwLock
        //         #field_ident: ::fieldx::RwLock::new(self.#field_ident.take().unwrap())
        //     ]
        // }
        else {
            self.simple_field_build_setter(fctx, field_ident, span)
        })
    }

    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_reader() {
            let reader_name = self.reader_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Reader);
            let ty = fctx.ty();
            let attributes_fn = fctx.attributes_fn(fctx.reader().as_ref());

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #reader_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::MappedRwLockReadGuard<'fx_reader_lifetime, #ty> {
                        self.#ident.read()
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #reader_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::MappedRwLockReadGuard<'fx_reader_lifetime, #ty> {
                        ::fieldx::RwLockReadGuard::map(self.#ident.read(), |data: &Option<#ty>| data.as_ref().unwrap())
                    }
                ])
            }
            else {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #reader_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::RwLockReadGuard<#ty> {
                        self.#ident.read()
                    }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_writer() {
            let writer_name = self.writer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Writer);
            let ty = fctx.ty();
            let attributes_fn = fctx.attributes_fn(fctx.writer().as_ref());
            let proxy_type = self.type_tokens(fctx);

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #writer_name<'fx_writer_lifetime>(&'fx_writer_lifetime self) -> ::fieldx::FXWrLock<'fx_writer_lifetime, #ty, #proxy_type> {
                        self.#ident.write()
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #writer_name(&self) -> ::fieldx::RwLockWriteGuard<::std::option::Option<#ty>> {
                        self.#ident.write()
                    }
                ])
            }
            else {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #writer_name(&self) -> ::fieldx::RwLockWriteGuard<#ty> {
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
        // eprintln!("??? FIELD {} needs setter (from field:{:?}, args:{:?}): {:?}", fctx.ident_str(), fctx.field().needs_setter(), fctx.codegen_ctx().args().needs_setter(), fctx.needs_setter());
        if fctx.needs_setter() {
            let set_name = self.setter_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Setter);
            let ty = fctx.ty();
            let attributes_fn = fctx.attributes_fn(fctx.setter().as_ref());

            if fctx.is_lazy() {
                Ok(quote_spanned! [*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #set_name(&self, value: #ty) -> ::std::option::Option<#ty> {
                        self.#ident.write().store(value)
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #set_name(&self, value: #ty) -> ::std::option::Option<#ty> {
                        self.#ident.write().replace(value)
                    }
                ])
            }
            else if fctx.needs_lock() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #set_name(&self, value: #ty) -> #ty {
                        let mut wlock = self.#ident.write();
                        ::std::mem::replace(&mut *wlock, value)
                    }
                ])
            }
            else {
                Ok(quote_spanned! [*fctx.span()=>
                    #attributes_fn
                    #vis_tok fn #set_name(&mut self, value: #ty) -> #ty {
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
            let vis_tok = fctx.vis_tok(FXHelperKind::Clearer);
            let ty = fctx.ty();
            let attributes_fn = fctx.attributes_fn(fctx.clearer().as_ref());

            if fctx.is_lazy() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
                        self.#ident.clear()
                    }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
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
            let vis_tok = fctx.vis_tok(FXHelperKind::Predicate);
            let attributes_fn = fctx.attributes_fn(fctx.predicate().as_ref());

            if fctx.is_lazy() {
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #pred_name(&self) -> bool {
                        self.#ident.is_set()
                    }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![*fctx.span()=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #pred_name(&self) -> bool {
                      self.#ident.read().is_some()
                   }
                ])
            }
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: TokenStream) -> darling::Result<TokenStream> {
        if fctx.is_lazy() {
            let ty_tok = fctx.ty_tok();
            if value.is_empty() {
                Ok(quote![::std::default::Default::default()])
            }
            else {
                let proxy_type = self.field_proxy_type(fctx);
                Ok(quote![ ::fieldx::#proxy_type::<#ty_tok>::from(#value) ])
            }
        }
        else if fctx.is_optional() {
            let value_tok = if value.is_empty() {
                quote![::std::option::Option::None]
            }
            else {
                quote![::std::option::Option::Some(#value)]
            };
            Ok(quote![ ::fieldx::RwLock::new(#value_tok) ])
        }
        else if fctx.needs_lock() {
            Ok(quote![ ::fieldx::RwLock::new(#value) ])
        }
        else {
            Ok(quote![ #value ])
        }
    }

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.field_value_wrap(fctx, self.field_default_value(fctx)?)
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

    #[cfg(feature = "serde")]
    fn field_extras(&self, fctx: &FXFieldCtx) {
        if fctx.is_serde() {
            self.shadow_field(fctx);
            self.shadow_field_default(fctx);
        }
    }

    fn struct_extras(&'f self) {
        let ctx = self.ctx();
        let _sg = ctx.push_state(FXGenStage::MainExtras);
        let arc_self = self.arc_self();
        let initializers = self.initializer_toks.borrow_mut();
        let generics = ctx.input().generics();
        let generic_params = self.generic_params();
        let input = ctx.input_ident();
        let where_clause = &generics.where_clause;

        #[cfg(feature = "serde")]
        {
            self.shadow_struct();
            self.struct_from_shadow();
        }

        ctx.tokens_extend(quote![
            use ::fieldx::FXProxyCore;
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

        if ctx.args().needs_new() {
            self.add_method_decl(quote![
                #[inline]
                pub fn new() -> ::fieldx::Arc<Self> {
                    Self::default().__fieldx_init()
                }
            ])
        }
    }
}
