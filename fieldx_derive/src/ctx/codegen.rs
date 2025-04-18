use super::FXFieldCtx;
use crate::codegen::constructor::FXConstructor;
use crate::codegen::constructor::FXFieldConstructor;
use crate::codegen::constructor::FXFnConstructor;
use crate::codegen::constructor::FXStructConstructor;
use crate::field_receiver::FXField;
use crate::input_receiver::FXInputReceiver;
use crate::util::args::FXArgProps;
use crate::util::args::FXSArgs;
use crate::util::args::{self};
use delegate::delegate;
use fieldx_aux::FXProp;
use getset::CopyGetters;
use getset::Getters;
use once_cell::unsync::OnceCell;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::ToTokens;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;
#[derive(Debug, Getters, CopyGetters)]
pub(crate) struct FXCodeGenCtx {
    myself: Weak<Self>,

    errors: OnceCell<RefCell<darling::error::Accumulator>>,
    tokens: OnceCell<RefCell<TokenStream>>,

    user_struct:    RefCell<FXStructConstructor>,
    builder_struct: OnceCell<RefCell<FXStructConstructor>>,

    #[cfg(feature = "serde")]
    shadow_struct: RefCell<Option<FXStructConstructor>>,

    // #[cfg(feature = "serde")]
    // shadow_field_toks:   RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_default_toks: RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_var_ident:    OnceCell<syn::Ident>,
    #[cfg(feature = "serde")]
    me_var_ident:        OnceCell<syn::Ident>,

    #[getset(get = "pub(crate)")]
    args: FXSArgs,

    #[getset(skip)]
    arg_props:    Rc<FXArgProps>,
    // We use Option because FXInputReceiver can't implement Default
    input:        Option<Rc<FXInputReceiver>>,
    extra_fields: RefCell<Vec<FXField>>,
    unique_id:    RefCell<u32>,

    field_ctx_table: OnceCell<RefCell<HashMap<syn::Ident, Rc<FXFieldCtx>>>>,
    copyable_types:  RefCell<Vec<syn::Type>>,
}

impl FXCodeGenCtx {
    delegate! {
        /// Delegate to the FXArgProps implementation.
        to self.arg_props {
            pub(crate) fn needs_default(&self) -> FXProp<bool>;
            pub(crate) fn syncish(&self) -> FXProp<bool>;
            pub(crate) fn needs_new(&self) -> FXProp<bool>;
        }
    }

    pub(crate) fn new(input: FXInputReceiver, args: args::FXSArgs) -> Rc<Self> {
        let input = Rc::new(input);

        let mut user_struct = FXStructConstructor::new(input.ident().clone());
        user_struct
            .set_span(input.ident().span())
            .set_generics(input.generics().clone())
            .set_vis(input.vis().clone())
            .add_attributes(input.attrs().iter());

        Rc::new_cyclic(|myself| Self {
            myself: myself.clone(),
            arg_props: Rc::new(FXArgProps::new(args.clone(), myself.clone())),
            args,

            user_struct: RefCell::new(user_struct),
            builder_struct: OnceCell::new(),

            copyable_types: RefCell::new(Vec::new()),
            errors: OnceCell::new(),
            extra_fields: RefCell::new(Vec::new()),
            field_ctx_table: OnceCell::new(),
            tokens: OnceCell::new(),
            unique_id: RefCell::new(0),

            #[cfg(feature = "serde")]
            shadow_default_toks: RefCell::new(Vec::new()),
            #[cfg(feature = "serde")]
            shadow_var_ident: OnceCell::new(),
            #[cfg(feature = "serde")]
            me_var_ident: OnceCell::new(),

            #[cfg(feature = "serde")]
            shadow_struct: RefCell::new(None),

            input: Some(input),
        })
    }

    pub(crate) fn user_struct(&self) -> Ref<FXStructConstructor> {
        self.user_struct.borrow()
    }

    pub(crate) fn user_struct_mut(&self) -> RefMut<FXStructConstructor> {
        self.user_struct.borrow_mut()
    }

    pub(crate) fn builder_struct(&self) -> darling::Result<Ref<FXStructConstructor>> {
        Ok(self._builder_struct()?.borrow())
    }

    pub(crate) fn builder_struct_mut(&self) -> darling::Result<RefMut<FXStructConstructor>> {
        Ok(self._builder_struct()?.borrow_mut())
    }

    #[cfg(feature = "serde")]
    pub(crate) fn set_shadow_struct(&self, shadow_struct: FXStructConstructor) {
        self.shadow_struct.replace(Some(shadow_struct));
    }

    #[cfg(feature = "serde")]
    pub(crate) fn shadow_struct(&self) -> darling::Result<Ref<FXStructConstructor>> {
        let sstruct = self.shadow_struct.borrow();
        if sstruct.is_none() {
            return Err(darling::Error::custom("Shadow struct is not set yet").into());
        }
        Ok(Ref::map(sstruct, |s| s.as_ref().unwrap()))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn shadow_struct_mut(&self) -> darling::Result<RefMut<FXStructConstructor>> {
        let sstruct = self.shadow_struct.borrow_mut();
        if sstruct.is_none() {
            return Err(darling::Error::custom("Shadow struct is not set yet").into());
        }
        Ok(RefMut::map(sstruct, |s| s.as_mut().unwrap()))
    }

    fn myself(&self) -> Rc<Self> {
        self.myself
            .upgrade()
            .expect("Context object is gone while trying to upgrade a weak reference")
    }

    #[inline(always)]
    pub(crate) fn input(&self) -> &FXInputReceiver {
        // Unwrap is safe because the field is always set by the constructor and can't be taken away.
        self.input.as_ref().unwrap()
    }

    pub(crate) fn arg_props(&self) -> &Rc<FXArgProps> {
        &self.arg_props
    }

    #[inline(always)]
    pub(crate) fn copyable_types(&self) -> std::cell::Ref<Vec<syn::Type>> {
        self.copyable_types.borrow()
    }

    // #[inline(always)]
    // #[cfg(feature = "serde")]
    // pub fn shadow_fields(&self) -> std::cell::Ref<Vec<TokenStream>> {
    //     self.shadow_field_toks.borrow()
    // }

    #[inline(always)]
    #[cfg(feature = "serde")]
    pub(crate) fn shadow_defaults(&self) -> std::cell::Ref<Vec<TokenStream>> {
        self.shadow_default_toks.borrow()
    }

    #[inline(always)]
    pub(crate) fn push_error(&self, err: darling::Error) {
        self._errors().borrow_mut().push(err);
    }

    #[inline(always)]
    pub(crate) fn ok_or_empty(&self, outcome: darling::Result<TokenStream>) -> TokenStream {
        outcome.unwrap_or_else(|err| {
            self.push_error(err);
            quote![]
        })
    }

    #[inline(always)]
    pub(crate) fn ok_or_record<T>(&self, outcome: darling::Result<T>) {
        if let Err(err) = outcome {
            self.push_error(err);
        }
    }

    pub(crate) fn exec_or_record(&self, code: impl FnOnce() -> darling::Result<()>) {
        if let Err(err) = code() {
            self.push_error(err)
        }
    }

    #[inline(always)]
    pub(crate) fn add_field_decl(&self, field: FXFieldConstructor) {
        self.user_struct.borrow_mut().add_field(field);
    }

    #[inline(always)]
    pub(crate) fn add_method(&self, method: FXFnConstructor) -> &Self {
        self.user_struct.borrow_mut().struct_impl_mut().add_method(method);
        self
    }

    #[inline(always)]
    pub(crate) fn maybe_add_method(&self, method: Option<FXFnConstructor>) -> &Self {
        if let Some(method) = method {
            self.user_struct.borrow_mut().struct_impl_mut().add_method(method);
        }
        self
    }

    #[inline(always)]
    pub(crate) fn add_builder_method(&self, builder: FXFnConstructor) -> darling::Result<&Self> {
        self.builder_struct_mut()?.struct_impl_mut().add_method(builder);
        Ok(self)
    }

    #[inline(always)]
    pub(crate) fn add_builder_field(&self, builder_field: FXFieldConstructor) -> darling::Result<&Self> {
        self.builder_struct_mut()?.add_field(builder_field);
        Ok(self)
    }

    #[inline(always)]
    pub(crate) fn add_for_copy_trait_check(&self, field_ctx: &FXFieldCtx) {
        self.copyable_types.borrow_mut().push(field_ctx.ty().clone());
    }

    #[inline(always)]
    #[cfg(feature = "serde")]
    pub(crate) fn add_shadow_default_decl(&self, field: TokenStream) {
        self.shadow_default_toks.borrow_mut().push(field);
    }

    #[inline(always)]
    pub(crate) fn input_ident(&self) -> &syn::Ident {
        &self.input().ident()
    }

    pub(crate) fn tokens_extend<T: ToTokens>(&self, toks: T) {
        self._tokens().borrow_mut().extend(toks.to_token_stream());
    }

    #[inline]
    pub(crate) fn finalize(&self) -> TokenStream {
        if let Some(errors) = self.errors.get().map(|e| e.take()) {
            match errors.finish() {
                Ok(_) => (),
                Err(err) => {
                    self.tokens_extend(TokenStream::from(darling::Error::from(err).write_errors()));
                }
            };
        }

        if let Some(tokens) = self.tokens.get() {
            tokens.take()
        }
        else {
            quote![]
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    // How to reference shadow instance in an associated function
    pub(crate) fn shadow_var_ident(&self) -> &syn::Ident {
        self.shadow_var_ident
            .get_or_init(|| format_ident!("__shadow", span = self.arg_props().serde().final_span()))
    }

    // How to reference struct instance in an associated function
    #[cfg(feature = "serde")]
    #[inline]
    pub(crate) fn me_var_ident(&self) -> &syn::Ident {
        self.me_var_ident
            .get_or_init(|| format_ident!("__me", span = self.arg_props().serde().final_span()))
    }

    #[inline]
    pub(crate) fn add_extra_field(&self, field: FXField) {
        self.extra_fields.borrow_mut().push(field);
    }

    pub(crate) fn field_ctx_table(&self) -> &RefCell<HashMap<syn::Ident, Rc<FXFieldCtx>>> {
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

    pub(crate) fn ident_field_ctx(self: &Rc<Self>, field_ident: &syn::Ident) -> darling::Result<Rc<FXFieldCtx>> {
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

    pub(crate) fn field_ctx(&self, field: &FXField) -> Rc<FXFieldCtx> {
        let mut field_ctx_table = self.field_ctx_table().borrow_mut();
        field_ctx_table
            .entry(field.ident().unwrap().clone())
            .or_insert_with(|| Rc::new(FXFieldCtx::new(field.clone(), self.myself())))
            .clone()
    }

    pub(crate) fn all_field_ctx(&self) -> Vec<Rc<FXFieldCtx>> {
        // Don't iterate over field_ctx_table keys because we want to preserve the order of fields as they appear in the
        // struct.
        self.extra_fields
            .borrow()
            .iter()
            .chain(self.input().fields().iter().cloned())
            .map(|f| self.field_ctx(f))
            .collect()
    }

    #[inline(always)]
    pub(crate) fn struct_generic_params(&self) -> TokenStream {
        self.input().generics().split_for_impl().1.to_token_stream()
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn unique_ident_pfx(&self, prefix: &str) -> syn::Ident {
        let new_count = *self.unique_id.borrow() + 1;
        let _ = self.unique_id.replace(new_count);
        format_ident!("{}_{}", prefix, new_count)
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn unique_ident(&self) -> syn::Ident {
        self.unique_ident_pfx(&format!("__{}_fxsym", self.input_ident()))
    }

    fn _builder_struct(&self) -> darling::Result<&RefCell<FXStructConstructor>> {
        let arg_props = self.arg_props();
        let prop = arg_props.builder_struct();
        if *prop {
            Ok(self.builder_struct.get_or_init(|| {
                let builder_struct = RefCell::new(FXStructConstructor::new(arg_props.builder_ident().clone()));
                {
                    let mut bs_mut = builder_struct.borrow_mut();
                    bs_mut
                        .set_vis(arg_props.builder_struct_visibility())
                        .set_generics(self.input().generics().clone())
                        .set_span(prop.final_span())
                        .maybe_add_attributes(arg_props.builder_struct_attributes().map(|a| a.iter()))
                        .struct_impl_mut()
                        .maybe_add_attributes(arg_props.builder_struct_attributes_impl().map(|a| a.iter()));
                }

                builder_struct
            }))
        }
        else {
            Err(darling::Error::custom("Builder struct is not enabled").into())
        }
    }

    #[inline(always)]
    fn _errors(&self) -> &RefCell<darling::error::Accumulator> {
        self.errors.get_or_init(|| RefCell::new(darling::Error::accumulator()))
    }

    #[inline(always)]
    fn _tokens(&self) -> &RefCell<TokenStream> {
        self.tokens.get_or_init(|| RefCell::new(TokenStream::new()))
    }
}
