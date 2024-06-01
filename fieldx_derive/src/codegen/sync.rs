use crate::codegen::context::{FXCodeGenCtx, FXFieldCtx};
#[cfg(feature = "serde")]
use crate::codegen::FXCGenSerde;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    iter::Iterator,
};
use syn::spanned::Spanned;

use super::{FXCGen, FXCGenContextual, FXHelperKind};

pub(crate) struct FXCodeGen<'f> {
    ctx:                 FXCodeGenCtx,
    field_ctx_table:     RefCell<HashMap<syn::Ident, FXFieldCtx<'f>>>,
    field_toks:          RefCell<Vec<TokenStream>>,
    default_toks:        RefCell<Vec<TokenStream>>,
    method_toks:         RefCell<Vec<TokenStream>>,
    builder_field_toks:  RefCell<Vec<TokenStream>>,
    builder_toks:        RefCell<Vec<TokenStream>>,
    builder_field_ident: RefCell<Vec<syn::Ident>>,
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

impl<'f> FXCodeGen<'f> {
    #[inline]
    fn field_proxy_type(&self, _fctx: &FXFieldCtx) -> TokenStream {
        quote![FXProxy]
    }

    fn field_reader_method(&self, fctx: &FXFieldCtx, helper_kind: FXHelperKind) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, helper_kind);
        let method_name = self.helper_name(fctx, helper_kind)?;
        let ident = fctx.ident_tok();
        let vis_tok = fctx.vis_tok(helper_kind);
        let ty = fctx.ty();
        let attributes_fn = self.attributes_fn(fctx, helper_kind);

        if fctx.is_lazy() {
            Ok(quote_spanned! [span=>
                #[inline(always)]
                #attributes_fn
                #vis_tok fn #method_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::MappedRwLockReadGuard<'fx_reader_lifetime, #ty> {
                    self.#ident.read(self)
                }
            ])
        }
        else if fctx.is_optional() {
            Ok(quote_spanned! [span=>
                #[inline]
                #attributes_fn
                #vis_tok fn #method_name<'fx_reader_lifetime>(&'fx_reader_lifetime self) -> ::fieldx::RwLockReadGuard<'fx_reader_lifetime, ::std::option::Option<#ty>> {
                    self.#ident.read()
                    // ::fieldx::RwLockReadGuard::map(self.#ident.read(), |data: &Option<#ty>| data.as_ref().unwrap())
                }
            ])
        }
        else {
            Ok(quote_spanned! [span=>
                #[inline(always)]
                #attributes_fn
                #vis_tok fn #method_name(&self) -> ::fieldx::RwLockReadGuard<#ty> {
                    self.#ident.read()
                }
            ])
        }
    }
}

impl<'f> FXCGenContextual<'f> for FXCodeGen<'f> {
    #[inline(always)]
    fn ctx(&self) -> &FXCodeGenCtx {
        &self.ctx
    }

    #[inline(always)]
    fn fxstruct_trait(&self) -> TokenStream {
        quote![::fieldx::traits::FXStructSync]
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

    fn add_for_copy_trait_check(&self, fctx: &FXFieldCtx) {
        self.copyable_types.borrow_mut().push(fctx.ty().clone());
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

    fn defaults_combined(&self) -> TokenStream {
        let default_toks = self.default_toks.borrow();
        quote! [ #( #default_toks ),* ]
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
            let ident = self.ctx().input_ident();
            let generic_params = self.generic_params();
            let mut ty_toks = ty_tok.clone();

            if !fctx.is_skipped() {
                if fctx.is_lazy() {
                    let proxy_type = self.field_proxy_type(fctx);
                    ty_toks = quote_spanned! [span=> ::fieldx::#proxy_type<#ident #generic_params, #ty_tok>];
                }
                else if fctx.is_optional() {
                    ty_toks = quote_spanned! [span=> ::fieldx::FXRwLock<Option<#ty_tok>>];
                }
                else if fctx.needs_lock() {
                    ty_toks = quote_spanned! [span=> ::fieldx::FXRwLock<#ty_tok>]
                }
            }
            ty_toks
        })
    }

    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_accessor() {
            let is_copy = fctx.is_copy();
            let is_clone = fctx.is_clone();
            let is_lazy = fctx.is_lazy();
            let is_optional = fctx.is_optional();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Accessor);
            let span = self.helper_span(fctx, FXHelperKind::Accessor);
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let accessor_name = self.accessor_name(fctx)?;
            let ty = fctx.ty();

            if is_clone || is_copy {
                if is_lazy || fctx.needs_lock() || is_optional {
                    let ty = if is_optional { quote![Option<#ty>] } else { quote![#ty] };

                    let read_arg = if is_lazy { quote![self] } else { quote![] };

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

                    Ok(quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            let rlock = self.#ident.read(#read_arg);
                            (*rlock)#cmethod
                        }
                    ])
                }
                else if is_clone || is_copy {
                    let cmethod = if fctx.is_copy() { quote![] } else { quote![.clone()] };

                    Ok(quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> #ty {
                            self.#ident #cmethod
                        }
                    ])
                }
                else {
                    Ok(quote_spanned![span=>
                        #attributes_fn
                        #vis_tok fn #accessor_name(&self) -> & #ty {
                            & self.#ident
                        }
                    ])
                }
            }
            else if is_lazy || is_optional || fctx.needs_lock() {
                self.field_reader_method(fctx, FXHelperKind::Accessor)
            }
            else {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> &#ty {
                        &self.#ident
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
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Accessor);
            let accessor_name = self.accessor_mut_name(fctx)?;
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Accessor);
            let span = self.helper_span(fctx, FXHelperKind::Accessor);

            if fctx.is_lazy() {
                Ok(quote_spanned! [span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #accessor_name<'fx_get_mut>(&'fx_get_mut self) -> ::fieldx::MappedRwLockWriteGuard<'fx_get_mut, #ty> {
                        self.#ident.read_mut(self)
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned! [span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #accessor_name<'fx_get_mut>(&'fx_get_mut self) -> ::fieldx::MappedRwLockWriteGuard<'fx_get_mut, #ty> {
                        ::fieldx::RwLockWriteGuard::map(self.#ident.write(), |data: &Option<#ty>| data.as_ref().unwrap())
                    }
                ])
            }
            else if fctx.needs_lock() {
                Ok(quote_spanned! [span=>
                    #[inline(always)]
                    #attributes_fn
                    #vis_tok fn #accessor_name(&self) -> ::fieldx::RwLockWriteGuard<#ty> {
                        self.#ident.write()
                    }
                ])
            }
            else {
                // Bare field
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #accessor_name(&mut self) -> &mut #ty {
                        &mut self.#ident
                    }
                ])
            }
        }
        else {
            Ok(quote![])
        }
    }

    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let span = self.helper_span(fctx, FXHelperKind::Builder);
        let field_ident = fctx.ident_tok();
        let input_type = self.input_type_toks();
        let field_type = self.type_tokens(fctx);
        let lazy_builder_name = self.lazy_name(fctx)?;
        let or_default = if fctx.has_default_value() {
            let default = self.fixup_self_type(self.field_default_value(fctx)?.expect(&format!(
                "Internal problem: expected default value for field {}",
                fctx.ident_str()
            )));
            quote![  .or_else(|| Some(#default)) ]
        }
        else {
            quote![]
        };

        Ok(if fctx.is_ignorable() || !fctx.needs_builder() {
            quote![]
        }
        else if fctx.is_lazy() {
            quote_spanned![span=>
                #field_ident: <#field_type>::new_default(
                    <#input_type>::#lazy_builder_name,
                    self.#field_ident.take()#or_default
                )
            ]
        }
        else if fctx.is_optional() {
            quote_spanned![span=>
                // Optional can simply pickup and own either Some() or None from the builder.
                #field_ident: ::fieldx::FXRwLock::new(self.#field_ident.take()#or_default)
            ]
        }
        else {
            self.simple_field_build_setter(fctx, field_ident, &span)
        })
    }

    fn field_lazy_initializer(
        &self,
        fctx: &FXFieldCtx,
        self_ident: Option<TokenStream>,
    ) -> darling::Result<TokenStream> {
        let ident = fctx.ident_tok();
        let self_var = self_ident.unwrap_or(quote![self]);
        Ok(quote![#self_var.#ident.lazy_init(#self_var)])
    }

    #[cfg(feature = "serde")]
    fn field_from_shadow(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let shadow_var = self.ctx().shadow_var_ident();
        self.field_value_wrap(fctx, Some(quote![ #shadow_var.#field_ident ]))
    }

    #[cfg(feature = "serde")]
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let field_ident = fctx.ident_tok();
        let me_var = self.ctx().me_var_ident();
        Ok(if self.is_serde_optional(fctx) || fctx.needs_lock() {
            quote![ #me_var.#field_ident.into_inner() ]
        }
        else {
            quote![ #me_var.#field_ident ]
        })
    }

    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_reader() {
            self.field_reader_method(fctx, FXHelperKind::Reader)
        }
        else {
            Ok(TokenStream::new())
        }
    }

    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_writer() {
            let span = self.helper_span(fctx, FXHelperKind::Writer);
            let writer_name = self.writer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Writer);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Writer);

            if fctx.is_lazy() {
                Ok(quote_spanned! [span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #writer_name<'fx_writer_lifetime>(&'fx_writer_lifetime self) -> ::fieldx::FXWrLock<'fx_writer_lifetime, Self, #ty> {
                        self.#ident.write()
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #writer_name(&self) -> ::fieldx::RwLockWriteGuard<::std::option::Option<#ty>> {
                        self.#ident.write()
                    }
                ])
            }
            else {
                Ok(quote_spanned![span=>
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
            let span = self.helper_span(fctx, FXHelperKind::Setter);
            let set_name = self.setter_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Setter);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Setter);
            let (gen_params, val_type, into_tok) = self.into_toks(fctx, fctx.is_setter_into());

            if fctx.is_lazy() {
                Ok(quote_spanned! [span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident.write().store(value #into_tok)
                    }
                ])
            }
            else if fctx.is_optional() {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&self, value: #val_type) -> ::std::option::Option<#ty> {
                        self.#ident.write().replace(value #into_tok)
                    }
                ])
            }
            else if fctx.needs_lock() {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&self, value: #val_type) -> #ty {
                        let mut wlock = self.#ident.write();
                        ::std::mem::replace(&mut *wlock, value #into_tok)
                    }
                ])
            }
            else {
                Ok(quote_spanned! [span=>
                    #attributes_fn
                    #vis_tok fn #set_name #gen_params(&mut self, value: #val_type) -> #ty {
                        ::std::mem::replace(&mut self.#ident, value #into_tok)
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
            let span = self.helper_span(fctx, FXHelperKind::Clearer);
            let clear_name = self.clearer_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Clearer);
            let ty = fctx.ty();
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Clearer);

            if fctx.is_lazy() {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #clear_name(&self) -> ::std::option::Option<#ty> {
                        self.#ident.clear()
                    }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![span=>
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
            let span = self.helper_span(fctx, FXHelperKind::Predicate);
            let pred_name = self.predicate_name(fctx)?;
            let ident = fctx.ident_tok();
            let vis_tok = fctx.vis_tok(FXHelperKind::Predicate);
            let attributes_fn = self.attributes_fn(fctx, FXHelperKind::Predicate);

            if fctx.is_lazy() {
                Ok(quote_spanned![span=>
                    #[inline]
                    #attributes_fn
                    #vis_tok fn #pred_name(&self) -> bool {
                        self.#ident.is_set()
                    }
                ])
            }
            else {
                // If not lazy then it's optional
                Ok(quote_spanned![span=>
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

    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: Option<TokenStream>) -> darling::Result<TokenStream> {
        Ok(if fctx.is_lazy() {
            let input_type = self.input_type_toks();
            let field_type = self.type_tokens(fctx);
            let lazy_builder_name = self.lazy_name(fctx)?;

            value.map_or_else(
                || quote![ <#field_type>::new_default(<#input_type>::#lazy_builder_name, ::std::option::Option::None) ],
                |value| quote![ <#field_type>::new_default(<#input_type>::#lazy_builder_name, #value) ],
            )
        }
        else if fctx.is_optional() {
            let value_tok = value.map_or_else(|| quote![::std::option::Option::None], |value| quote![#value]);
            quote![ ::fieldx::FXRwLock::new(#value_tok) ]
        }
        else {
            let value = value.map(|value| quote![ #value]).ok_or_else(|| {
                darling::Error::custom(format!(
                    "No value was supplied for non-optional, non-lazy field {}",
                    fctx.ident_str()
                ))
            })?;

            if fctx.needs_lock() {
                quote![ ::fieldx::FXRwLock::new(#value) ]
            }
            else {
                quote![ #value ]
            }
        })
    }

    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.field_value_wrap(
            fctx,
            self.field_default_value(fctx)?.map(|d| {
                if fctx.is_optional() || fctx.is_lazy() {
                    quote![ ::std::option::Option::Some(#d) ]
                }
                else {
                    quote![ #d ]
                }
            }),
        )
    }
}
