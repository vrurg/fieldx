use super::{Attributizer, FXFieldCtx};
use crate::{
    fields::FXField,
    helper::{FXHelperContainer, FXHelperKind, FXOrig},
    input_receiver::FXInputReceiver,
    util::args::{self, FXArgProps, FXSArgs},
};
use delegate::delegate;
use fieldx_aux::{FXHelperTrait, FXProp};
use getset::{CopyGetters, Getters};
use once_cell::unsync::OnceCell;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};
use syn::{spanned::Spanned, Ident};

#[derive(Debug, Getters, CopyGetters)]
pub struct FXCodeGenCtx {
    myself: Weak<Self>,

    errors:             RefCell<OnceCell<darling::error::Accumulator>>,
    is_builder_opt_in:  RefCell<OnceCell<bool>>,
    tokens:             RefCell<OnceCell<TokenStream>>,
    field_toks:         RefCell<Vec<TokenStream>>,
    default_toks:       RefCell<Vec<TokenStream>>,
    method_toks:        RefCell<Vec<TokenStream>>,
    builder_field_toks: RefCell<Vec<TokenStream>>,
    builder_toks:       RefCell<Vec<TokenStream>>,

    #[cfg(feature = "serde")]
    shadow_field_toks:   RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_default_toks: RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_var_ident:    OnceCell<syn::Ident>,
    #[cfg(feature = "serde")]
    me_var_ident:        OnceCell<syn::Ident>,

    #[getset(get = "pub")]
    args: FXSArgs,

    #[getset(skip)]
    arg_props:    Rc<FXArgProps>,
    // We use Option because FXInputReceiver can't implement Default
    input:        Option<Rc<FXInputReceiver>>,
    extra_attrs:  RefCell<Vec<syn::Attribute>>,
    extra_fields: RefCell<Vec<FXField>>,
    unique_id:    RefCell<u32>,

    field_ctx_table:     OnceCell<RefCell<HashMap<syn::Ident, Rc<FXFieldCtx>>>>,
    builder_field_ident: RefCell<Vec<syn::Ident>>,
    copyable_types:      RefCell<Vec<syn::Type>>,

    is_syncish: RefCell<OnceCell<bool>>,
}

impl FXCodeGenCtx {
    delegate! {
        /// Delegate to the FXArgProps implementation.
        to self.arg_props {
            pub fn needs_default(&self) -> FXProp<bool>;
            pub fn syncish(&self) -> FXProp<bool>;
            pub fn needs_new(&self) -> FXProp<bool>;
        }
    }

    pub fn new(input: FXInputReceiver, args: args::FXSArgs) -> Rc<Self> {
        let input = Rc::new(input);
        Rc::new_cyclic(|myself| Self {
            myself: myself.clone(),
            arg_props: Rc::new(FXArgProps::new(args.clone(), myself.upgrade().unwrap())),
            input: Some(input),
            args,

            builder_field_ident: RefCell::new(Vec::new()),
            builder_field_toks: RefCell::new(Vec::new()),
            builder_toks: RefCell::new(Vec::new()),
            copyable_types: RefCell::new(Vec::new()),
            default_toks: RefCell::new(Vec::new()),
            errors: RefCell::new(OnceCell::new()),
            extra_attrs: RefCell::new(Vec::new()),
            extra_fields: RefCell::new(Vec::new()),
            field_ctx_table: OnceCell::new(),
            field_toks: RefCell::new(Vec::new()),
            is_builder_opt_in: RefCell::new(OnceCell::new()),
            is_syncish: RefCell::new(OnceCell::new()),
            method_toks: RefCell::new(Vec::new()),
            tokens: RefCell::new(OnceCell::new()),
            unique_id: RefCell::new(0),

            #[cfg(feature = "serde")]
            shadow_field_toks: RefCell::new(Vec::new()),
            #[cfg(feature = "serde")]
            shadow_default_toks: RefCell::new(Vec::new()),
            #[cfg(feature = "serde")]
            shadow_var_ident: OnceCell::new(),
            #[cfg(feature = "serde")]
            me_var_ident: OnceCell::new(),
        })
    }

    fn myself(&self) -> Rc<Self> {
        self.myself
            .upgrade()
            .expect("Context object is gone while trying to upgrade a weak reference")
    }

    #[inline(always)]
    pub fn input(&self) -> &FXInputReceiver {
        // Unwrap is safe because the field is always set by the constructor and can't be taken away.
        self.input.as_ref().unwrap()
    }

    pub fn arg_props(&self) -> &Rc<FXArgProps> {
        &self.arg_props
    }

    #[inline(always)]
    pub fn builder_field_ident(&self) -> &RefCell<Vec<syn::Ident>> {
        &self.builder_field_ident
    }

    #[inline(always)]
    pub fn copyable_types(&self) -> std::cell::Ref<Vec<syn::Type>> {
        self.copyable_types.borrow()
    }

    #[inline(always)]
    #[cfg(feature = "serde")]
    pub fn shadow_fields(&self) -> std::cell::Ref<Vec<TokenStream>> {
        self.shadow_field_toks.borrow()
    }

    #[inline(always)]
    #[cfg(feature = "serde")]
    pub fn shadow_defaults(&self) -> std::cell::Ref<Vec<TokenStream>> {
        self.shadow_default_toks.borrow()
    }

    pub fn push_error(&self, err: darling::Error) {
        let mut errors = self.errors.borrow_mut();
        errors.get_or_init(|| darling::Error::accumulator());
        errors.get_mut().unwrap().push(err);
    }

    #[inline(always)]
    pub fn ok_or_else<T>(&self, outcome: darling::Result<T>, mapper: impl FnOnce() -> T) -> T {
        outcome.unwrap_or_else(|err| {
            self.push_error(err);
            mapper()
        })
    }

    #[inline(always)]
    pub fn ok_or_empty(&self, outcome: darling::Result<TokenStream>) -> TokenStream {
        self.ok_or_else(outcome, || quote![])
    }

    #[inline(always)]
    pub fn ok_or_record(&self, outcome: darling::Result<()>) {
        if let Err(err) = outcome {
            self.push_error(err)
        }
    }

    pub fn exec_or_record(&self, code: impl FnOnce() -> darling::Result<()>) {
        if let Err(err) = code() {
            self.push_error(err)
        }
    }

    #[inline(always)]
    pub fn add_field_decl(&self, field: TokenStream) {
        self.field_toks.borrow_mut().push(field);
    }

    #[inline(always)]
    pub fn add_defaults_decl(&self, defaults: TokenStream) {
        self.default_toks.borrow_mut().push(defaults);
    }

    #[inline(always)]
    pub fn add_method_decl(&self, method: TokenStream) {
        if !method.is_empty() {
            self.method_toks.borrow_mut().push(method);
        }
    }

    #[inline(always)]
    pub fn add_builder_decl(&self, builder: TokenStream) {
        if !builder.is_empty() {
            self.builder_toks.borrow_mut().push(builder);
        }
    }

    #[inline(always)]
    pub fn add_builder_field_decl(&self, builder_field: TokenStream) {
        if !builder_field.is_empty() {
            self.builder_field_toks.borrow_mut().push(builder_field);
        }
    }

    #[inline(always)]
    pub fn add_builder_field_ident(&self, field_ident: syn::Ident) {
        self.builder_field_ident.borrow_mut().push(field_ident);
    }

    #[inline(always)]
    pub fn add_for_copy_trait_check(&self, field_ctx: &FXFieldCtx) {
        self.copyable_types.borrow_mut().push(field_ctx.ty().clone());
    }

    #[cfg(feature = "serde")]
    pub fn add_shadow_field_decl(&self, field: TokenStream) {
        self.shadow_field_toks.borrow_mut().push(field);
    }

    #[inline(always)]
    #[cfg(feature = "serde")]
    pub fn add_shadow_default_decl(&self, field: TokenStream) {
        self.shadow_default_toks.borrow_mut().push(field);
    }

    #[inline(always)]
    pub fn input_ident(&self) -> &syn::Ident {
        &self.input().ident()
    }

    pub fn tokens_extend(&self, toks: TokenStream) {
        let mut tokens = self.tokens.borrow_mut();
        tokens.get_or_init(|| TokenStream::new());
        tokens.get_mut().unwrap().extend(toks);
    }

    pub fn methods_combined(&self) -> TokenStream {
        let method_toks = self.method_toks.borrow();
        quote! [ #( #method_toks )* ]
    }

    pub fn struct_fields(&self) -> Ref<Vec<TokenStream>> {
        self.field_toks.borrow()
    }

    pub fn builders_combined(&self) -> TokenStream {
        let builder_toks = &*self.builder_toks.borrow();
        quote! [
            #( #builder_toks )*
        ]
    }

    pub fn builder_fields_combined(&self) -> TokenStream {
        let build_field_toks = &*self.builder_field_toks.borrow();
        quote! [ #( #build_field_toks ),* ]
    }

    pub fn defaults_combined(&self) -> Option<TokenStream> {
        let default_toks = &*self.default_toks.borrow();
        if default_toks.is_empty() {
            None
        }
        else {
            Some(quote_spanned! [self.needs_default().final_span()=> #( #default_toks ),* ])
        }
    }

    #[inline]
    pub fn finalize(&self) -> TokenStream {
        let mut errors = self.errors.borrow_mut();
        match errors.take() {
            Some(errs) => match errs.finish() {
                Ok(_) => (),
                Err(err) => {
                    self.tokens_extend(TokenStream::from(darling::Error::from(err).write_errors()));
                }
            },
            None => (),
        };

        self.tokens.borrow_mut().take().unwrap_or_else(|| TokenStream::new())
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn shadow_ident(&self) -> syn::Ident {
        let span = self.args().serde_helper_span();
        if let Some(custom_name) = self.args.serde().as_ref().and_then(|s| s.shadow_name()) {
            quote::format_ident!(
                "{}",
                custom_name.value(),
                span = custom_name.orig_span().unwrap_or(span)
            )
        }
        else {
            quote::format_ident!("__{}Shadow", self.input_ident(), span = span)
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    // How to reference shadow instance in an associated function
    pub fn shadow_var_ident(&self) -> &syn::Ident {
        self.shadow_var_ident
            .get_or_init(|| format_ident!("__shadow", span = self.arg_props().serde().final_span()))
    }

    // How to reference struct instance in an associated function
    #[cfg(feature = "serde")]
    #[inline]
    pub fn me_var_ident(&self) -> &syn::Ident {
        self.me_var_ident
            .get_or_init(|| format_ident!("__me", span = self.arg_props().serde().final_span()))
    }

    #[allow(dead_code)]
    #[inline]
    pub fn add_attr_from<ATTR: ToTokens>(&self, attr: ATTR) {
        let Some(attr) = Attributizer::from(attr).into_inner()
        else {
            return;
        };
        self.extra_attrs.borrow_mut().push(attr);
    }

    #[allow(dead_code)]
    #[inline]
    pub fn add_attr(&self, attr: syn::Attribute) {
        self.extra_attrs.borrow_mut().push(attr);
    }

    #[inline]
    pub fn add_extra_field(&self, field: FXField) {
        self.extra_fields.borrow_mut().push(field);
    }

    #[inline]
    pub fn all_attrs(&self) -> Vec<syn::Attribute> {
        let mut attrs: Vec<syn::Attribute> = self
            .extra_attrs
            .borrow()
            .iter()
            .chain(self.input().attrs().iter())
            .cloned()
            .collect();
        attrs.sort_by(|a, b| {
            if a.path().is_ident("derive") && !b.path().is_ident("derive") {
                std::cmp::Ordering::Less
            }
            else if !a.path().is_ident("derive") && b.path().is_ident("derive") {
                std::cmp::Ordering::Greater
            }
            else {
                std::cmp::Ordering::Equal
            }
        });
        attrs
    }

    pub fn field_ctx_table(&self) -> &RefCell<HashMap<syn::Ident, Rc<FXFieldCtx>>> {
        self.field_ctx_table.get_or_init(|| {
            RefCell::new(
                self.extra_fields
                    .borrow()
                    .iter()
                    .chain(self.input().fields().iter().cloned())
                    .map(|f| {
                        let field_ident = f.ident().unwrap();
                        (field_ident.clone(), Rc::new(FXFieldCtx::new(f.clone(), self.myself())))
                    })
                    .collect(),
            )
        })
    }

    pub fn ident_field_ctx(self: &Rc<Self>, field_ident: &syn::Ident) -> darling::Result<Rc<FXFieldCtx>> {
        Ok(self
            .field_ctx_table()
            .borrow()
            .get(field_ident)
            .ok_or(darling::Error::custom(format!(
                "Field '{}' not found in context table",
                field_ident
            )))?
            .clone())
    }

    pub fn field_ctx(&self, field: &FXField) -> Rc<FXFieldCtx> {
        let mut field_ctx_table = self.field_ctx_table().borrow_mut();
        field_ctx_table
            .entry(field.ident().unwrap().clone())
            .or_insert_with(|| Rc::new(FXFieldCtx::new(field.clone(), self.myself())))
            .clone()
    }

    pub fn all_field_ctx(&self) -> Vec<Rc<FXFieldCtx>> {
        // Don't iterate over field_ctx_table keys because we want to preserve the order of fields as they appear in the
        // struct.
        self.extra_fields
            .borrow()
            .iter()
            .chain(self.input().fields().iter().cloned())
            .map(|f| self.field_ctx(f))
            .collect()
    }

    // #[inline]
    // pub fn helper_span(&self, helper_kind: FXHelperKind) -> Span {
    //     self.args()
    //         .get_helper_span(helper_kind)
    //         .unwrap_or_else(|| Span::call_site())
    // }

    #[inline(always)]
    pub fn struct_generic_params(&self) -> TokenStream {
        self.input().generics().split_for_impl().1.to_token_stream()
    }

    #[allow(dead_code)]
    #[inline]
    pub fn unique_ident_pfx(&self, prefix: &str) -> syn::Ident {
        let new_count = *self.unique_id.borrow() + 1;
        let _ = self.unique_id.replace(new_count);
        format_ident!("{}_{}", prefix, new_count)
    }

    #[allow(dead_code)]
    #[inline]
    pub fn unique_ident(&self) -> syn::Ident {
        self.unique_ident_pfx(&format!("__{}_fxsym", self.input_ident()))
    }
}
