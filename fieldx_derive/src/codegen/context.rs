use super::{
    FXAccessorMode, FXAttributes, FXBoolArg, FXBuilder, FXHelperContainer, FXHelperKind, FXHelperTrait, FXPubMode,
};
#[cfg(feature = "serde")]
use crate::helper::FXSerde;
use crate::{
    fields::FXField,
    helper::with_origin::FXOrig,
    input_receiver::FXInputReceiver,
    util::args::{self, FXSArgs},
};
use darling::ast::NestedMeta;
use delegate::delegate;
use getset::{CopyGetters, Getters};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::cell::{OnceCell, RefCell};

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

struct Attributizer(Option<syn::Attribute>);

impl Attributizer {
    fn into_inner(self) -> Option<syn::Attribute> {
        self.0
    }
}

impl<T> From<T> for Attributizer
where
    T: ToTokens,
{
    fn from(attr: T) -> Self {
        let toks = attr.to_token_stream();
        if toks.is_empty() {
            Self(None)
        }
        else {
            Self(Some(syn::parse_quote!(#attr)))
        }
    }
}

// --- Contexts ---

#[derive(Debug, Getters, CopyGetters)]
pub(crate) struct FXCodeGenCtx {
    errors:               RefCell<OnceCell<darling::error::Accumulator>>,
    needs_builder_struct: RefCell<Option<bool>>,
    tokens:               RefCell<OnceCell<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_var_ident:     OnceCell<syn::Ident>,
    #[cfg(feature = "serde")]
    me_var_ident:         OnceCell<syn::Ident>,
    #[getset(get = "pub")]
    args:                 FXSArgs,
    #[getset(get = "pub")]
    input:                FXInputReceiver,
    extra_attrs:          RefCell<Vec<syn::Attribute>>,
    unique_id:            RefCell<u32>,
    needs_default:        RefCell<OnceCell<bool>>,
}

#[derive(Debug)]
pub(crate) struct FXFieldCtx<'f> {
    field:            &'f FXField,
    codegen_ctx:      &'f FXCodeGenCtx,
    ty_tok:           OnceCell<TokenStream>,
    ty_wrapped:       OnceCell<TokenStream>,
    ident:            OnceCell<darling::Result<&'f syn::Ident>>,
    ident_tok:        OnceCell<TokenStream>,
    #[cfg(feature = "serde")]
    default_fn_ident: OnceCell<darling::Result<syn::Ident>>,
}

impl FXCodeGenCtx {
    pub fn new(input: FXInputReceiver, args: args::FXSArgs) -> Self {
        Self {
            input,
            args,
            errors: RefCell::new(OnceCell::new()),
            tokens: RefCell::new(OnceCell::new()),
            #[cfg(feature = "serde")]
            me_var_ident: OnceCell::new(),
            #[cfg(feature = "serde")]
            shadow_var_ident: OnceCell::new(),
            needs_builder_struct: RefCell::new(None),
            extra_attrs: RefCell::new(vec![]),
            unique_id: RefCell::new(0),
            needs_default: RefCell::new(OnceCell::new()),
        }
    }

    pub fn push_error(&self, err: darling::Error) {
        let mut errors = self.errors.borrow_mut();
        errors.get_or_init(|| darling::Error::accumulator());
        errors.get_mut().unwrap().push(err);
    }

    pub fn input_ident(&self) -> &syn::Ident {
        &self.input.ident()
    }

    pub fn tokens_extend(&self, toks: TokenStream) {
        let mut tokens = self.tokens.borrow_mut();
        tokens.get_or_init(|| TokenStream::new());
        tokens.get_mut().unwrap().extend(toks);
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

    #[inline]
    pub fn needs_builder_struct(&self) -> Option<bool> {
        (*self.needs_builder_struct.borrow()).or(self.args.needs_builder())
    }

    #[inline]
    pub fn require_builder(&self) {
        let mut nb_ref = self.needs_builder_struct.borrow_mut();
        if nb_ref.is_none() {
            *nb_ref = Some(true);
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn shadow_ident(&self) -> syn::Ident {
        if let Some(custom_name) = self.args.serde().as_ref().and_then(|s| s.shadow_name()) {
            quote::format_ident!("{}", custom_name)
        }
        else {
            quote::format_ident!("__{}Shadow", self.input_ident())
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    // How to reference shadow instance in an associated function
    pub fn shadow_var_ident(&self) -> &syn::Ident {
        self.shadow_var_ident.get_or_init(|| format_ident!("__shadow"))
    }

    // How to reference struct instance in an associated function
    #[cfg(feature = "serde")]
    #[inline]
    pub fn me_var_ident(&self) -> &syn::Ident {
        self.me_var_ident.get_or_init(|| format_ident!("__me"))
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

    #[inline]
    pub fn helper_span(&self, helper_kind: FXHelperKind) -> Span {
        self.args()
            .get_helper_span(helper_kind)
            .unwrap_or_else(|| Span::call_site())
    }

    pub fn needs_default(&self) -> bool {
        let args = self.args();

        *self.needs_default.borrow_mut().get_or_init(|| {
            if let Some(needs_default) = args.needs_default() {
                return needs_default;
            }

            if args.needs_new() {
                return true;
            }

            let is_sync = args.is_sync();

            if is_sync && args.is_lazy().unwrap_or(false) {
                return true;
            }

            if self
                .input()
                .fields()
                .iter()
                .any(|f| f.has_default_value() || (is_sync && f.is_lazy().unwrap_or(false)))
            {
                return true;
            }

            false
        })
    }
}

impl<'f> FXFieldCtx<'f> {
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

    helper_fn_ctx! { is: lazy }

    helper_fn_ctx! { needs: accessor_mut, builder, setter, writer }

    arg_accessor! { optional: FXBoolArg, lock: FXBoolArg }

    pub fn new(field: &'f FXField, codegen_ctx: &'f FXCodeGenCtx) -> Self {
        Self {
            field,
            codegen_ctx,
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            ident_tok: OnceCell::new(),
            ty_tok: OnceCell::new(),
            #[cfg(feature = "serde")]
            default_fn_ident: OnceCell::new(),
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
            .or_else(|| self.codegen_ctx.args.needs_accessor())
            .unwrap_or_else(|| self.needs_clearer() || self.needs_predicate() || self.is_lazy())
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
            .or_else(|| self.codegen_ctx.args.needs_predicate())
            .unwrap_or(false)
    }

    #[inline]
    pub fn needs_reader(&self) -> bool {
        self.field
            .needs_reader()
            .or_else(|| self.codegen_ctx.args.needs_reader())
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

    pub fn is_copy(&self) -> bool {
        self.field
            .is_accessor_copy()
            .or_else(|| self.field.is_copy())
            .or_else(|| self.codegen_ctx().args().is_accessor_copy())
            .or_else(|| self.codegen_ctx().args().is_copy())
            .unwrap_or(false)
    }

    pub fn is_clone(&self) -> bool {
        self.field
            .is_accessor_clone()
            .or_else(|| self.field.is_clone())
            .or_else(|| self.codegen_ctx().args().is_accessor_clone())
            .or_else(|| self.codegen_ctx().args().is_clone())
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
        !self.is_skipped()
            && self
                .field
                .is_optional()
                .or_else(|| self.codegen_ctx.args.is_optional())
                .unwrap_or_else(|| (!self.is_lazy() && (self.needs_clearer() || self.needs_predicate())))
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

    pub fn default_value(&self) -> Option<&NestedMeta> {
        if self.field.has_default_value() {
            self.field.default_value().as_ref().and_then(|dv| dv.value().as_ref())
        }
        else {
            None
        }
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
    pub fn ident(&self) -> darling::Result<&syn::Ident> {
        self.ident.get_or_init(|| self.field.ident()).clone()
    }

    #[inline]
    pub fn ident_tok(&self) -> &TokenStream {
        self.ident_tok.get_or_init(|| {
            self.ident()
                .as_ref()
                .map_or_else(|err| err.clone().write_errors(), |i| i.to_token_stream())
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

    #[cfg(feature = "serde")]
    pub fn default_fn_ident<'s>(&'s self) -> darling::Result<&'s syn::Ident> {
        self.default_fn_ident
            .get_or_init(|| {
                let ctx = self.codegen_ctx();
                let field_ident = self.ident()?;
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

    pub fn all_attrs(&self) -> Vec<syn::Attribute> {
        self.attrs().iter().cloned().collect()
    }
}
