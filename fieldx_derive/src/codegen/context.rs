#[cfg(feature = "serde")]
use crate::helper::FXSerde;
use crate::{
    fields::FXField,
    helper::FXHelper,
    input_receiver::FXInputReceiver,
    util::args::{self, FXSArgs},
};
use delegate::delegate;
use getset::{CopyGetters, Getters};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::{
    cell::{OnceCell, Ref, RefCell, RefMut},
    rc::Rc,
};
use syn::Meta;

use super::{
    FXAccessor, FXAccessorMode, FXAttributes, FXFieldBuilder, FXHelperContainer, FXHelperKind, FXHelperTrait,
    FXNestingAttr, FXPubMode, FXSetter, FromNestAttr,
};

// For FXFieldCtx
macro_rules! helper_fn_ctx {
    ($prefix:ident: $( $field:ident ),+ ) => {
        ::paste::paste! {
            $(
                pub fn [<$prefix _ $field>](&self) -> bool {
                    self.field
                        .[<$prefix _ $field>]()
                        .or_else(|| self.codegen_ctx().args().[<$prefix _ $field>]())
                        .unwrap_or(false)
                }
            )+
        }
    };
}

// --- State Stack ---
#[derive(Debug, PartialEq)]
pub(crate) enum FXGenStage {
    Builder,
    BuilderImpl,
    MainDefaultImpl,
    MainStruct,
    MainExtras,
    ShadowStruct,
}

#[derive(Debug)]
pub(crate) struct FXStateStack<T>
where
    T: Sized + PartialEq,
{
    stack: Rc<RefCell<Vec<T>>>,
}

pub(crate) struct FXSStackGuard<T>
where
    T: Sized + PartialEq,
{
    stack: Option<Rc<RefCell<Vec<T>>>>,
    idx:   usize,
}

impl<T> FXStateStack<T>
where
    T: Sized + PartialEq,
{
    pub(crate) fn new() -> Self {
        Self {
            stack: Rc::new(RefCell::new(vec![])),
        }
    }

    #[inline]
    pub(crate) fn push(&self, val: T) -> FXSStackGuard<T> {
        FXSStackGuard::new(Rc::clone(&self.stack), val)
    }

    #[inline]
    pub(crate) fn in_state(&self, val: &T) -> bool {
        self.stack.borrow().contains(&val)
    }
}

impl<T: Sized> FXSStackGuard<T>
where
    T: Sized + PartialEq,
{
    pub(crate) fn new(stack: Rc<RefCell<Vec<T>>>, val: T) -> Self {
        let idx = { stack.borrow().len() };
        stack.borrow_mut().push(val);
        Self {
            stack: Some(stack),
            idx,
        }
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if let Some(stack) = self.stack.take() {
            let mut stack = stack.borrow_mut();
            if stack.len() > (self.idx + 1) {
                panic!(
                    "Attempt to pop while not being the top of the stack: my index is {} but the stack size is {}",
                    self.idx,
                    stack.len()
                );
            }
            else if stack.len() <= self.idx {
                panic!(
                    "Stack element with index {} does not exist: stack size is {}. Has it been popped already?",
                    self.idx,
                    stack.len()
                );
            }

            stack.pop()
        }
        else {
            None
        }
    }
}

impl<T> Drop for FXSStackGuard<T>
where
    T: Sized + PartialEq,
{
    fn drop(&mut self) {
        let _ = self.pop();
    }
}

// --- Contexts ---

#[derive(Debug, Getters, CopyGetters)]
pub(crate) struct FXCodeGenCtx {
    errors:               RefCell<OnceCell<darling::error::Accumulator>>,
    needs_builder_struct: RefCell<Option<bool>>,
    tokens:               RefCell<OnceCell<TokenStream>>,
    #[getset(get = "pub")]
    args:                 FXSArgs,
    #[getset(get = "pub")]
    input:                FXInputReceiver,
    #[getset(get = "pub")]
    state_stack:          FXStateStack<FXGenStage>,
}

#[derive(Debug)]
pub(crate) struct FXFieldCtx<'f> {
    field:       &'f FXField,
    codegen_ctx: &'f FXCodeGenCtx,
    ty_tok:      OnceCell<TokenStream>,
    ty_wrapped:  OnceCell<TokenStream>,
    ident:       OnceCell<Option<&'f syn::Ident>>,
    ident_tok:   OnceCell<TokenStream>,
}

impl FXCodeGenCtx {
    pub fn new(input: FXInputReceiver, args: args::FXSArgs) -> Self {
        Self {
            input,
            args,
            errors: RefCell::new(OnceCell::new()),
            tokens: RefCell::new(OnceCell::new()),
            needs_builder_struct: RefCell::new(None),
            state_stack: FXStateStack::new(),
        }
    }

    pub fn push_error(&self, err: darling::Error) {
        let mut errors = self.errors.borrow_mut();
        errors.get_or_init(|| darling::Error::accumulator());
        errors.get_mut().unwrap().push(err);
    }

    #[inline]
    pub fn push_state(&self, state: FXGenStage) -> FXSStackGuard<FXGenStage> {
        self.state_stack.push(state)
    }

    pub fn in_state(&self, state: FXGenStage) -> bool {
        self.state_stack.in_state(&state)
    }

    pub fn input_ident(&self) -> &syn::Ident {
        &self.input.ident()
    }

    pub fn tokens_extend(&self, toks: TokenStream) {
        let mut tokens = self.tokens.borrow_mut();
        tokens.get_or_init(|| TokenStream::new());
        tokens.get_mut().unwrap().extend(toks);
    }

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

    pub fn needs_builder_struct(&self) -> Option<bool> {
        (*self.needs_builder_struct.borrow()).or(self.args.needs_builder())
    }

    pub fn require_builder(&self) {
        let mut nb_ref = self.needs_builder_struct.borrow_mut();
        if nb_ref.is_none() {
            *nb_ref = Some(true);
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn shadow_ident(&self) -> syn::Ident {
        quote::format_ident!("__{}Shadow", self.input_ident())
    }
}

impl<'f> FXFieldCtx<'f> {
    delegate! {
        to self.field {
            pub fn is_ignorable(&self) -> bool;
            pub fn has_default_value(&self) -> bool;
            pub fn span(&self) -> &Span;
            pub fn vis(&self) -> &syn::Visibility;
            pub fn ty(&self) -> &syn::Type;
            pub fn attrs(&self) -> &Vec<syn::Attribute>;
            pub fn lazy(&self) -> &Option<FXHelper>;
            pub fn base_name(&self) -> &Option<String>;
            pub fn accessor(&self) -> &Option<FXAccessor>;
            pub fn accessor_mut(&self) -> &Option<FXHelper>;
            pub fn setter(&self) -> &Option<FXSetter>;
            pub fn builder(&self) -> &Option<FXFieldBuilder>;
            pub fn reader(&self) -> &Option<FXHelper>;
            pub fn writer(&self) -> &Option<FXHelper>;
            pub fn clearer(&self) -> &Option<FXHelper>;
            pub fn predicate(&self) -> &Option<FXHelper>;
            #[cfg(feature = "serde")]
            pub fn serde(&self) -> &Option<FXSerde>;
            pub fn default_value(&self) -> &Option<Meta>;
            pub fn builder_attributes(&self) -> Option<&FXAttributes>;
            pub fn builder_fn_attributes(&self) -> Option<&FXAttributes>;
            pub fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait>;
        }
    }

    helper_fn_ctx! {is: lazy}

    helper_fn_ctx! {needs: accessor_mut, builder, setter, writer}

    pub fn new(field: &'f FXField, codegen_ctx: &'f FXCodeGenCtx) -> Self {
        Self {
            field,
            codegen_ctx,
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            ident_tok: OnceCell::new(),
            ty_tok: OnceCell::new(),
        }
    }

    pub fn codegen_ctx(&self) -> &FXCodeGenCtx {
        &self.codegen_ctx
    }

    pub fn needs_accessor(&self) -> bool {
        self.field
            .needs_accessor()
            .or_else(|| self.codegen_ctx.args.needs_accessor())
            .unwrap_or_else(|| {
                // sync struct doesn't provide accessors by default.
                !self.codegen_ctx.args.is_sync() && (self.needs_clearer() || self.needs_predicate() || self.is_lazy())
            })
    }

    pub fn needs_clearer(&self) -> bool {
        self.field
            .needs_clearer()
            .or_else(|| self.codegen_ctx().args().needs_clearer())
            .unwrap_or(false)
    }

    pub fn needs_predicate(&self) -> bool {
        self.field
            .needs_predicate()
            .or_else(|| self.codegen_ctx.args.needs_predicate())
            .unwrap_or(false)
    }

    pub fn needs_reader(&self) -> bool {
        self.field
            .needs_reader()
            .or_else(|| self.codegen_ctx.args.needs_reader())
            .unwrap_or_else(|| self.codegen_ctx.args().is_sync() && (self.is_lazy() || self.is_optional()))
    }

    #[inline]
    pub fn needs_lock(&self) -> bool {
        self.needs_reader() || self.needs_writer()
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn needs_serialize(&self) -> bool {
        self.field
            .needs_serialize()
            .unwrap_or_else(|| self.codegen_ctx().args().is_serde())
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn needs_deserialize(&self) -> bool {
        self.field
            .needs_deserialize()
            .unwrap_or_else(|| self.codegen_ctx().args().is_serde())
    }

    pub fn is_copy(&self) -> bool {
        self.field
            .is_accessor_copy()
            .or_else(|| self.field.is_copy())
            .or_else(|| self.codegen_ctx().args().is_accessor_copy())
            .or_else(|| self.codegen_ctx().args().is_copy())
            .unwrap_or(false)
    }

    pub fn is_builder_into(&self) -> bool {
        self.field
            .is_builder_into()
            .or_else(|| self.field.is_into())
            .or_else(|| self.codegen_ctx().args().is_builder_into())
            .or_else(|| self.codegen_ctx.args.is_into())
            .unwrap_or(false)
    }

    pub fn is_setter_into(&self) -> bool {
        self.field
            .is_setter_into()
            .or_else(|| self.field.is_into())
            .or_else(|| self.codegen_ctx().args().is_setter_into())
            .or_else(|| self.codegen_ctx.args.is_into())
            .unwrap_or(false)
    }

    pub fn is_optional(&self) -> bool {
        !self.is_lazy() && (self.needs_clearer() || self.needs_predicate())
    }

    #[cfg(feature = "serde")]
    pub fn is_serde(&self) -> bool {
        self.codegen_ctx.args().is_serde() && self.field.is_serde().unwrap_or(true)
    }

    #[cfg(not(feature = "serde"))]
    #[inline]
    pub fn is_serde(&self) -> bool {
        false
    }

    pub fn accessor_mode(&self) -> FXAccessorMode {
        self.field
            .accessor_mode()
            .or_else(|| {
                self.field.is_copy().and_then(|is_copy| {
                    if is_copy {
                        Some(FXAccessorMode::Copy)
                    }
                    else {
                        Some(FXAccessorMode::Clone)
                    }
                })
            })
            .or_else(|| self.codegen_ctx.args.accessor_mode())
            .or_else(|| {
                self.codegen_ctx.args.is_copy().and_then(|is_copy| {
                    if is_copy {
                        Some(FXAccessorMode::Copy)
                    }
                    else {
                        Some(FXAccessorMode::Clone)
                    }
                })
            })
            .unwrap_or(FXAccessorMode::None)
    }

    #[allow(dead_code)]
    pub fn attributes<'a>(
        &'a self,
        helper: Option<&'a FXNestingAttr<impl FXHelperTrait + FromNestAttr>>,
    ) -> Option<&'a FXAttributes> {
        helper.and_then(|h| h.attributes())
    }

    pub fn attributes_fn<'a>(
        &'a self,
        helper: Option<&'a FXNestingAttr<impl FXHelperTrait + FromNestAttr>>,
    ) -> Option<&'a FXAttributes> {
        helper.and_then(|h| h.attributes_fn().or_else(|| self.field.attributes_fn().as_ref()))
    }

    #[inline]
    pub fn field(&self) -> &'f FXField {
        self.field
    }

    pub fn vis_tok(&self, helper_kind: FXHelperKind) -> TokenStream {
        let ctx = self.codegen_ctx();
        let helper = self.get_helper(helper_kind);
        let public_mode = helper
            .and_then(|h| h.public_mode())
            .or_else(|| self.field.public_mode())
            .or_else(|| {
                let cg_helper = ctx.args().get_helper(helper_kind);
                cg_helper
                    .and_then(|h| h.public_mode())
                    .or_else(|| ctx.args().public_mode())
            });

        match public_mode {
            None => ctx.input().vis().to_token_stream(),
            Some(FXPubMode::Private) => quote![],
            Some(FXPubMode::All) => quote!(pub),
            Some(FXPubMode::Super) => quote!(pub(super)),
            Some(FXPubMode::Crate) => quote!(pub(crate)),
            Some(FXPubMode::InMod(ref path)) => quote!(pub(in #path)),
        }
    }

    #[inline]
    pub fn ty_tok(&self) -> &TokenStream {
        self.ty_tok.get_or_init(|| self.field.ty().to_token_stream())
    }

    #[inline]
    pub fn ty_wrapped<F>(&self, initializer: F) -> &TokenStream
    where
        F: FnOnce() -> TokenStream,
    {
        self.ty_wrapped.get_or_init(initializer)
    }

    #[inline]
    pub fn ident(&self) -> Option<&'f syn::Ident> {
        self.ident.get_or_init(|| self.field.ident().as_ref()).clone()
    }

    #[inline]
    pub fn ident_tok(&self) -> &TokenStream {
        self.ident_tok.get_or_init(|| match self.ident() {
            Some(ident) => ident.to_token_stream(),
            None => TokenStream::new(),
        })
    }

    #[allow(dead_code)]
    #[inline]
    pub fn ident_str(&self) -> String {
        let tok = self.ident_tok();
        if tok.is_empty() {
            "<anon>".into()
        }
        else {
            tok.to_string()
        }
    }

    pub fn helper_base_name(&self) -> Option<String> {
        if let Some(base_name) = self.base_name() {
            Some(base_name.clone())
        }
        else if let Some(ident) = self.field.ident() {
            Some(ident.to_string())
        }
        else {
            None
        }
    }
}
