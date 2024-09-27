use super::FXCodeGenCtx;
use crate::{
    fields::FXField,
    helper::{FXHelperContainer, FXHelperKind},
};
use darling::ast::NestedMeta;
use delegate::delegate;
#[cfg(feature = "serde")]
use fieldx_aux::FXSerde;
use fieldx_aux::{FXAccessorMode, FXAttributes, FXBoolArg, FXBuilder, FXHelperTrait, FXOrig, FXPubMode};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
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

macro_rules! arg_accessor {
    ( $( $arg:ident: $ty:ty ),+ ) => {
        $(
            pub fn $arg(&self) -> Option<&$ty> {
                self.field.$arg()
                    .as_ref()
                    .or_else(|| self.codegen_ctx().args().$arg().as_ref())
            }
        )+
    };
}

#[derive(Debug)]
pub struct FXFieldCtx {
    field:            FXField,
    codegen_ctx:      Rc<FXCodeGenCtx>,
    ty_tok:           OnceCell<TokenStream>,
    ty_wrapped:       OnceCell<TokenStream>,
    ident:            OnceCell<syn::Ident>,
    ident_tok:        OnceCell<TokenStream>,
    #[cfg(feature = "serde")]
    default_fn_ident: OnceCell<darling::Result<syn::Ident>>,
    builder_checker:  RefCell<Option<TokenStream>>,
}

impl FXFieldCtx {
    delegate! {
        to self.field {
            pub fn attributes_fn(&self) -> &Option<FXAttributes>;
            pub fn attrs(&self) -> &Vec<syn::Attribute>;
            pub fn base_name(&self) -> Option<syn::Ident>;
            pub fn builder(&self) -> &Option<FXBuilder>;
            pub fn fieldx_attr_span(&self) -> &Option<Span>;
            pub fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait>;
            pub fn get_helper_span(&self, kind: FXHelperKind) -> Option<Span>;
            pub fn has_default_value(&self) -> bool;
            pub fn is_ignorable(&self) -> bool;
            pub fn is_skipped(&self) -> bool;
            pub fn span(&self) -> &Span;
            pub fn ty(&self) -> &syn::Type;
            pub fn vis(&self) -> &syn::Visibility;
            #[cfg(feature = "serde")]
            pub fn serde(&self) -> &Option<FXSerde>;
        }
    }

    helper_fn_ctx! { is: lazy, inner_mut }

    helper_fn_ctx! { needs: accessor_mut, setter, writer }

    arg_accessor! { optional: FXBoolArg, lock: FXBoolArg, inner_mut: FXBoolArg }

    pub fn new(field: FXField, codegen_ctx: &Rc<FXCodeGenCtx>) -> Self {
        let codegen_ctx = Rc::clone(codegen_ctx);
        Self {
            field,
            codegen_ctx,
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            ident_tok: OnceCell::new(),
            ty_tok: OnceCell::new(),
            #[cfg(feature = "serde")]
            default_fn_ident: OnceCell::new(),
            builder_checker: RefCell::new(None),
        }
    }

    #[inline]
    pub fn codegen_ctx(&self) -> &FXCodeGenCtx {
        &self.codegen_ctx
    }

    #[inline]
    pub fn needs_accessor(&self) -> bool {
        self.field
            .needs_accessor()
            .or_else(|| self.codegen_ctx.args().needs_accessor())
            .unwrap_or_else(|| self.needs_clearer() || self.needs_predicate() || self.is_lazy() || self.is_inner_mut())
    }

    #[inline]
    pub fn needs_builder(&self) -> bool {
        let codegen_ctx = self.codegen_ctx();
        self.field.needs_builder().unwrap_or_else(|| {
            codegen_ctx
                .args()
                .needs_builder()
                .as_ref()
                .map_or(false, |needs| *needs && !codegen_ctx.is_builder_opt_in())
        })
    }

    #[inline]
    pub fn needs_clearer(&self) -> bool {
        self.field
            .needs_clearer()
            .or_else(|| self.codegen_ctx().args().needs_clearer())
            .unwrap_or(false)
    }

    #[inline]
    pub fn needs_predicate(&self) -> bool {
        self.field
            .needs_predicate()
            .or_else(|| self.codegen_ctx.args().needs_predicate())
            .unwrap_or(false)
    }

    #[inline]
    pub fn needs_reader(&self) -> bool {
        self.field
            .needs_reader()
            .or_else(|| self.codegen_ctx.args().needs_reader())
            .unwrap_or(false)
        // .unwrap_or_else(|| self.codegen_ctx.args().is_sync() && (self.is_lazy() || self.is_optional()))
    }

    #[inline]
    pub fn needs_lock(&self) -> bool {
        self.field
            .needs_lock()
            .or_else(|| self.codegen_ctx().args().needs_lock())
            .unwrap_or_else(|| self.needs_reader() || self.needs_writer() || self.is_optional())
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

    #[inline]
    pub fn forced_builder(&self) -> bool {
        self.builder().as_ref().map_or(false, |b| b.name().is_some())
    }

    #[inline]
    pub fn is_sync(&self) -> bool {
        self.field
            .is_sync()
            .unwrap_or_else(|| self.codegen_ctx().is_rather_sync())
    }

    #[inline]
    #[allow(dead_code)] // XXX Temporaryly, until asyncs are implemented
    pub fn is_async(&self) -> bool {
        self.field
            .is_async()
            .unwrap_or_else(|| self.codegen_ctx().args().is_async())
    }

    #[inline]
    pub fn is_copy(&self) -> bool {
        self.field
            .is_accessor_copy()
            .or_else(|| self.field.is_copy())
            .or_else(|| self.codegen_ctx().args().is_accessor_copy())
            .or_else(|| self.codegen_ctx().args().is_copy())
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_clone(&self) -> bool {
        self.field
            .is_accessor_clone()
            .or_else(|| self.field.is_clone())
            .or_else(|| self.codegen_ctx().args().is_accessor_clone())
            .or_else(|| self.codegen_ctx().args().is_clone())
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_builder_into(&self) -> bool {
        self.field
            .is_builder_into()
            .or_else(|| self.field.is_into())
            .or_else(|| self.codegen_ctx().args().is_builder_into())
            .or_else(|| self.codegen_ctx.args().is_into())
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_setter_into(&self) -> bool {
        self.field
            .is_setter_into()
            .or_else(|| self.field.is_into())
            .or_else(|| self.codegen_ctx().args().is_setter_into())
            .or_else(|| self.codegen_ctx.args().is_into())
            .unwrap_or(false)
    }

    pub fn is_optional(&self) -> bool {
        !self.is_skipped()
            && self
                .field
                .is_optional()
                .or_else(|| self.codegen_ctx.args().is_optional())
                .unwrap_or_else(|| (!self.is_lazy() && (self.needs_clearer() || self.needs_predicate())))
    }

    pub fn is_builder_required(&self) -> bool {
        self.builder()
            .as_ref()
            .and_then(|h| h.is_required())
            .or_else(|| self.codegen_ctx().args().is_builder_required())
            .unwrap_or(false)
    }

    #[cfg(feature = "serde")]
    pub fn is_serde(&self) -> bool {
        !self.is_skipped() && (self.codegen_ctx.args().is_serde() && self.field.is_serde().unwrap_or(true))
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
            .or_else(|| self.codegen_ctx.args().accessor_mode())
            .or_else(|| {
                self.codegen_ctx.args().is_copy().and_then(|is_copy| {
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

    pub fn default_value(&self) -> Option<&NestedMeta> {
        if self.field.has_default_value() {
            self.field.default_value().as_ref().and_then(|dv| dv.value().as_ref())
        }
        else {
            None
        }
    }

    #[inline]
    pub fn field(&self) -> &FXField {
        &self.field
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
    pub fn ident(&self) -> &syn::Ident {
        self.ident
            .get_or_init(|| self.field.ident().expect("No field ident found"))
    }

    #[inline]
    pub fn ident_tok(&self) -> &TokenStream {
        self.ident_tok.get_or_init(|| self.ident().to_token_stream())
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

    #[cfg(feature = "serde")]
    pub fn default_fn_ident<'s>(&'s self) -> darling::Result<&'s syn::Ident> {
        self.default_fn_ident
            .get_or_init(|| {
                let ctx = self.codegen_ctx();
                let field_ident = self.ident();
                let struct_ident = ctx.input_ident();
                Ok(ctx.unique_ident_pfx(&format!("__{}_{}_default", struct_ident, field_ident)))
            })
            .as_ref()
            .map_err(
                // Normally, cloning of the error would only take place once since the upstream would give up and won't try
                // requesting the ident again.
                |e| e.clone(),
            )
    }

    pub fn helper_base_name(&self) -> darling::Result<syn::Ident> {
        if let Some(bn) = self.base_name() {
            Ok(bn.clone())
        }
        else {
            Ok(self.field.ident()?.clone())
        }
    }

    pub fn helper_span(&self, helper_kind: FXHelperKind) -> Span {
        self.get_helper_span(helper_kind).unwrap_or_else(|| Span::call_site())
    }

    pub fn optional_span(&self) -> Span {
        self.optional()
            .map_or_else(
                || {
                    self.get_helper_span(FXHelperKind::Clearer)
                        .or_else(|| self.get_helper_span(FXHelperKind::Predicate))
                },
                |o| o.span(),
            )
            .unwrap_or_else(|| Span::call_site())
    }

    pub fn lock_span(&self) -> Span {
        self.lock().and_then(|l| l.span()).unwrap_or_else(|| {
            self.get_helper_span(FXHelperKind::Reader)
                .or_else(|| self.get_helper_span(FXHelperKind::Writer))
                .unwrap_or_else(|| self.optional_span())
        })
    }

    // Origin of span information almost follows the rules of finding out the mode information:
    // arguments of fieldx(get()) -> arguments of fieldx() -> arguments of fxstruct(get()) -> arguments of fxstruct()
    pub fn accessor_mode_span(&self) -> Option<Span> {
        self.field()
            .accessor_mode_span()
            .or_else(|| self.codegen_ctx().args().accessor_mode_span())
    }

    pub fn inner_mut_span(&self) -> Span {
        self.inner_mut()
            .and_then(|im| im.span())
            .unwrap_or_else(|| Span::call_site())
    }

    pub fn all_attrs(&self) -> Vec<syn::Attribute> {
        self.attrs().iter().cloned().collect()
    }

    pub fn set_builder_checker(&self, bc: TokenStream) {
        *self.builder_checker.borrow_mut() = Some(bc);
    }

    pub fn builder_checker(&self) -> Option<TokenStream> {
        self.builder_checker.borrow().clone()
    }
}
