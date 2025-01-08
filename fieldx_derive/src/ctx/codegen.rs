use super::{Attributizer, FXFieldCtx};
use crate::{
    fields::FXField,
    helper::{FXHelperContainer, FXHelperKind, FXOrig},
    input_receiver::FXInputReceiver,
    util::args::{self, FXSArgs},
};
use fieldx_aux::FXHelperTrait;
use getset::{CopyGetters, Getters};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::{
    cell::{OnceCell, Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};
use syn::{spanned::Spanned, Ident};

#[derive(Default, Debug, Getters, CopyGetters)]
pub struct FXCodeGenCtx {
    errors:               RefCell<OnceCell<darling::error::Accumulator>>,
    needs_builder_struct: RefCell<OnceCell<bool>>,
    is_builder_opt_in:    RefCell<OnceCell<bool>>,
    tokens:               RefCell<OnceCell<TokenStream>>,
    field_toks:           RefCell<Vec<TokenStream>>,
    default_toks:         RefCell<Vec<TokenStream>>,
    method_toks:          RefCell<Vec<TokenStream>>,
    builder_field_toks:   RefCell<Vec<TokenStream>>,
    builder_toks:         RefCell<Vec<TokenStream>>,

    #[cfg(feature = "serde")]
    shadow_field_toks:   RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_default_toks: RefCell<Vec<TokenStream>>,
    #[cfg(feature = "serde")]
    shadow_var_ident:    OnceCell<syn::Ident>,
    #[cfg(feature = "serde")]
    me_var_ident:        OnceCell<syn::Ident>,

    #[getset(get = "pub")]
    args:          FXSArgs,
    // We use Option because FXInputReceiver can't implement Default
    input:         Option<FXInputReceiver>,
    extra_attrs:   RefCell<Vec<syn::Attribute>>,
    extra_fields:  RefCell<Vec<FXField>>,
    unique_id:     RefCell<u32>,
    needs_default: RefCell<OnceCell<bool>>,

    field_ctx_table:     RefCell<HashMap<syn::Ident, FXFieldCtx>>,
    builder_field_ident: RefCell<Vec<syn::Ident>>,
    copyable_types:      RefCell<Vec<syn::Type>>,

    is_rather_sync: RefCell<OnceCell<bool>>,
}

impl FXCodeGenCtx {
    pub fn new(input: FXInputReceiver, args: args::FXSArgs) -> Self {
        Self {
            input: Some(input),
            args,
            ..Default::default()
        }
    }

    #[inline(always)]
    pub fn input(&self) -> &FXInputReceiver {
        // Unwrap is safe because the field is always set by the constructor and can't be taken away.
        self.input.as_ref().unwrap()
    }

    // #[inline(always)]
    // pub fn field_ctx_table(&self) -> Ref<HashMap<syn::Ident, FXFieldCtx>> {
    //     self.field_ctx_table.borrow()
    // }

    // #[inline(always)]
    // pub fn field_ctx_table_mut(&self) -> RefMut<HashMap<syn::Ident, FXFieldCtx>> {
    //     self.field_ctx_table.borrow_mut()
    // }

    #[inline(always)]
    pub fn builder_ident(&self) -> Ident {
        format_ident!(
            "{}{}",
            self.input_ident(),
            "Builder",
            span = self.helper_span(FXHelperKind::Builder)
        )
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

    pub fn defaults_combined(&self) -> TokenStream {
        let default_toks = &*self.default_toks.borrow();
        quote! [ #( #default_toks ),* ]
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
    pub fn is_builder_opt_in(&self) -> bool {
        *self
            .is_builder_opt_in
            .borrow_mut()
            .get_or_init(|| self.args().builder().as_ref().map_or(false, |b| b.is_builder_opt_in()))
    }

    #[inline]
    pub fn needs_builder_struct(&self) -> bool {
        *self.needs_builder_struct.borrow_mut().get_or_init(|| {
            self.args.needs_builder().unwrap_or_else(|| {
                let mut builder_required = false;
                for field in self.input().fields() {
                    if let Some(needs) = field.needs_builder() {
                        if needs {
                            let _ = self.is_builder_opt_in.borrow_mut().set(true);
                            builder_required = true;
                            break;
                        }
                    }
                }
                builder_required
            })
        })
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn shadow_ident(&self) -> syn::Ident {
        let span = self.args().serde_helper_span();
        if let Some(custom_name) = self.args.serde().as_ref().and_then(|s| s.shadow_name()) {
            quote::format_ident!("{}", custom_name, span = span)
        }
        else {
            quote::format_ident!("__{}Shadow", self.input_ident(), span = span)
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    // How to reference shadow instance in an associated function
    pub fn shadow_var_ident(&self) -> &syn::Ident {
        let span = self.args().serde_helper_span();
        self.shadow_var_ident
            .get_or_init(|| format_ident!("__shadow", span = span))
    }

    // How to reference struct instance in an associated function
    #[cfg(feature = "serde")]
    #[inline]
    pub fn me_var_ident(&self) -> &syn::Ident {
        let span = self.args().serde_helper_span();
        self.me_var_ident.get_or_init(|| format_ident!("__me", span = span))
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
    pub fn add_field(&self, field: FXField) {
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

    pub fn ident_field_ctx(&self, field_ident: &syn::Ident) -> darling::Result<Ref<FXFieldCtx>> {
        let fctx_table = self.field_ctx_table.borrow();
        Ref::filter_map(fctx_table, |ft| ft.get(field_ident))
            .map_err(|_| darling::Error::custom(format!("No context found for field `{}`", field_ident)))
    }

    pub fn field_ctx<'s>(self: &'s Rc<Self>, field: FXField) -> darling::Result<Ref<'s, FXFieldCtx>> {
        let field_ident = field.ident()?.clone();
        {
            let mut fctx_table = self.field_ctx_table.borrow_mut();
            if !fctx_table.contains_key(&field_ident) {
                let _ = fctx_table.insert(field_ident.clone(), <FXFieldCtx>::new(field, self));
            }
        }
        self.ident_field_ctx(&field_ident)
    }

    pub fn all_fields(&self) -> Vec<FXField> {
        self.extra_fields
            .borrow()
            .iter()
            .chain(self.input().fields().iter().cloned())
            .cloned()
            .collect()
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

            let is_sync = self.is_rather_sync();

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

    pub fn myself_field(&self) -> Option<Ident> {
        self.myself_names()
            .map(|(myself_name, _)| format_ident!("__weak_{}", myself_name))
    }

    pub fn myself_names(&self) -> Option<(Ident, Ident)> {
        self.args().rc().as_ref().map(|rc_helper| {
            let myself_name = rc_helper.name().unwrap_or("myself");
            (
                format_ident!("{}", myself_name),
                format_ident!("{}_downgrade", myself_name),
            )
        })
    }

    // Try to infer what mode applies to the struct. If it's explicitly declared as sync or async then there is no
    // doubt. Otherwise see if any field is asking for sync mode.
    pub fn is_rather_sync(&self) -> bool {
        *self.is_rather_sync.borrow_mut().get_or_init(|| {
            self.args.is_sync().map_or_else(
                || {
                    for field in self.all_fields() {
                        if let Some(is_sync) = field.is_sync() {
                            return is_sync;
                        }
                    }
                    false
                },
                |is_sync| is_sync,
            )
        })
    }

    pub fn builder_struct_visibility(&self) -> TokenStream {
        self.args()
            .get_helper(FXHelperKind::Builder)
            .and_then(|builder| builder.public_mode().map(|pm| pm.to_token_stream()))
            .or_else(|| Some(self.input().vis().to_token_stream()))
            .unwrap()
    }

    #[inline(always)]
    pub fn builder_has_post_build(&self) -> bool {
        self.args.builder().as_ref().map_or(false, |b| b.has_post_build())
    }

    #[inline(always)]
    pub fn builder_post_build_ident(&self) -> Option<syn::Ident> {
        if self.builder_has_post_build() {
            Some(
                self.args()
                    .builder()
                    .as_ref()
                    .and_then(|b| b.post_build().as_ref())
                    .and_then(|pb| pb.value().cloned())
                    .unwrap_or_else(|| {
                        let span = self
                            .args()
                            .builder()
                            .as_ref()
                            .and_then(|b| b.post_build().as_ref().unwrap().orig())
                            .map_or_else(|| self.helper_span(FXHelperKind::Builder), |o| o.span());
                        format_ident!("post_build", span = span)
                    }),
            )
        }
        else {
            None
        }
    }

    pub fn build_has_error_type(&self) -> bool {
        self.args.builder().as_ref().map_or(false, |b| b.error_type().is_some())
    }

    pub fn builder_error_type(&self) -> Option<&syn::Path> {
        self.args.builder().as_ref().and_then(|b| b.error_type())
    }

    pub fn builder_error_variant(&self) -> Option<&syn::Path> {
        self.args.builder().as_ref().and_then(|b| b.error_variant())
    }

    #[inline(always)]
    pub fn struct_generic_params(&self) -> TokenStream {
        self.input().generics().split_for_impl().1.to_token_stream()
    }
}
