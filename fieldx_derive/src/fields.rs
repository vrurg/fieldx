use crate::{
    helper::{FXHelperContainer, FXHelperKind},
    util::needs_helper,
};
use darling::{util::Flag, FromField};
use fieldx_aux::{
    validate_exclusives, FXAccessor, FXAccessorMode, FXAttributes, FXBaseHelper, FXBool, FXBoolHelper, FXBuilder,
    FXDefault, FXFallible, FXHelper, FXHelperTrait, FXNestingAttr, FXOrig, FXPubMode, FXSerde, FXSetter, FXString,
    FXSynValue, FXSyncMode, FXTriggerHelper, FromNestAttr,
};
use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use std::{cell::OnceCell, ops::Deref};
use syn::{spanned::Spanned, Meta};

#[derive(Debug, FromField, Getters, Clone)]
#[getset(get = "pub(crate)")]
#[darling(attributes(fieldx), forward_attrs)]
pub(crate) struct FXFieldReceiver {
    #[getset(skip)]
    ident: Option<syn::Ident>,
    vis:   syn::Visibility,
    ty:    syn::Type,
    attrs: Vec<syn::Attribute>,

    skip: Flag,

    #[getset(skip)]
    mode:       Option<FXSynValue<FXSyncMode>>,
    #[getset(skip)]
    #[darling(rename = "sync")]
    mode_sync:  Option<FXBool>,
    #[getset(skip)]
    #[darling(rename = "r#async")]
    mode_async: Option<FXBool>,

    // Default method attributes for this field.
    attributes_fn: Option<FXAttributes>,
    fallible:      Option<FXNestingAttr<FXFallible>>,
    lazy:          Option<FXHelper>,
    #[darling(rename = "rename")]
    #[getset(skip)]
    base_name:     Option<FXString>,
    #[darling(rename = "get")]
    accessor:      Option<FXAccessor>,
    #[darling(rename = "get_mut")]
    accessor_mut:  Option<FXHelper>,
    #[darling(rename = "set")]
    setter:        Option<FXSetter>,
    reader:        Option<FXHelper>,
    writer:        Option<FXHelper>,
    clearer:       Option<FXHelper>,
    predicate:     Option<FXHelper>,
    optional:      Option<FXBool>,

    public:        Option<FXNestingAttr<FXPubMode>>,
    private:       Option<FXBool>,
    #[darling(rename = "default")]
    default_value: Option<FXDefault>,
    builder:       Option<FXBuilder>,
    into:          Option<FXBool>,
    #[getset(get = "pub with_prefix")]
    clone:         Option<FXBool>,
    #[getset(get = "pub with_prefix")]
    copy:          Option<FXBool>,
    lock:          Option<FXBool>,
    inner_mut:     Option<FXBool>,
    serde:         Option<FXSerde>,

    #[darling(skip)]
    #[getset(skip)]
    span: OnceCell<Span>,

    #[darling(skip)]
    fieldx_attr_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub(crate) struct FXField(FXFieldReceiver);

impl FromField for FXField {
    fn from_field(field: &syn::Field) -> darling::Result<Self> {
        let mut fxfield = FXFieldReceiver::from_field(field)?;
        for attr in (&field.attrs).into_iter() {
            // Intercept #[fieldx] form of the attribute and mark the field manually
            if attr.path().is_ident("fieldx") {
                fxfield.set_attr_span(attr.span());

                if attr.meta.require_path_only().is_ok() {
                    fxfield.mark_implicitly(attr.meta.clone()).map_err(|err| {
                        darling::Error::custom(format!("Can't use bare word '{}'", err)).with_span(attr)
                    })?;
                }
            }
        }
        if let Err(_) = fxfield.set_span((field as &dyn Spanned).span()) {
            let err = darling::Error::custom("Can't set span for a field receiver object: it's been set already!")
                .with_span(field);
            #[cfg(feature = "diagnostics")]
            let err = err.note("This must not happen normally, please report this error to the author of fieldx");
            return Err(err);
        }
        fxfield.validate()?;
        Ok(Self(fxfield))
    }
}

impl Deref for FXField {
    type Target = FXFieldReceiver;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToTokens for FXField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fxr = &self.0;
        let FXFieldReceiver {
            ident, vis, ty, attrs, ..
        } = fxr;
        tokens.extend(quote_spanned![*fxr.span()=> #( #attrs )* #vis #ident: #ty])
    }
}

impl FXFieldReceiver {
    validate_exclusives! {
        "visibility": public; private;
        "accessor mode": copy; clone;
        "field mode":  lazy; optional;
        "in-/fallible mode": fallible; lock, optional, inner_mut;
        "concurrency mode": mode_sync as "sync"; mode_async as "async"; mode;
    }

    // Generate field-level needs_<helper> methods. The final decision of what's needed and what's not is done by
    // FXFieldCtx.
    needs_helper! {accessor, accessor_mut, builder, clearer, setter, predicate, reader, writer}

    pub fn validate(&self) -> darling::Result<()> {
        let mut acc = darling::Error::accumulator();

        if let Err(err) = self.validate_exclusives() {
            acc.push(err);
        }

        // XXX Make it a warning when possible.
        // if self.is_fallible().unwrap_or(false) && !self.is_lazy().unwrap_or(false) {
        //     return Err(
        //         darling::Error::custom("Parameter 'fallible' only makes sense when 'lazy' is set too")
        //             .with_span(&self.fallible().fx_span()),
        //     );
        // }

        #[cfg(not(feature = "sync"))]
        if let Some(err) = crate::util::feature_required("sync", &self.mode_sync) {
            acc.push(err);
        }

        #[cfg(not(feature = "async"))]
        if let Some(err) = crate::util::feature_required("async", &self.mode_async) {
            acc.push(err);
        }

        #[cfg(not(feature = "serde"))]
        if let Some(err) = crate::util::feature_required("serde", &self.serde) {
            acc.push(err);
        }

        acc.finish()?;

        Ok(())
    }

    #[inline]
    fn unless_skip(&self, helper: &Option<FXNestingAttr<impl FXHelperTrait + FromNestAttr>>) -> Option<bool> {
        if self.is_skipped() {
            Some(false)
        }
        else {
            helper.is_true_opt()
        }
    }

    #[inline(always)]
    pub fn public_mode(&self) -> Option<FXPubMode> {
        fieldx_aux::public_mode(&self.public, &self.private)
    }

    pub fn ident(&self) -> darling::Result<syn::Ident> {
        self.ident.clone().ok_or_else(|| {
            darling::Error::custom("This is weird, but the field doesn't have an ident!").with_span(self.span())
        })
    }

    pub fn base_name(&self) -> Option<syn::Ident> {
        if let Some(ref bn) = self.base_name {
            bn.value().map(|name| syn::Ident::new(name, bn.span()))
        }
        else {
            None
        }
    }

    #[inline]
    pub fn is_sync(&self) -> Option<bool> {
        self.mode_sync
            .as_ref()
            .map(|th| th.is_true())
            .or_else(|| self.mode.as_ref().map(|m| m.is_sync()))
    }

    #[inline]
    pub fn is_async(&self) -> Option<bool> {
        self.mode_async
            .as_ref()
            .map(|th| th.is_true())
            .or_else(|| self.mode.as_ref().map(|m| m.is_async()))
    }

    #[inline]
    pub fn is_plain(&self) -> Option<bool> {
        self.mode.as_ref().and_then(|m| Some(m.is_plain()))
    }

    /// Only ever return `Some(true)` if either `lock`, `writer`, or `reader` are set.
    #[inline]
    pub fn is_syncish(&self) -> Option<bool> {
        self.lock()
            .as_ref()
            .and_then(|l| l.true_or_none())
            .or_else(|| self.reader().as_ref().and_then(|r| r.true_or_none()))
            .or_else(|| self.writer().as_ref().and_then(|w| w.true_or_none()))
    }

    #[inline]
    pub fn is_fallible(&self) -> Option<bool> {
        self.fallible.is_true_opt()
    }

    #[inline]
    pub fn is_lazy(&self) -> Option<bool> {
        self.unless_skip(&self.lazy)
    }

    #[inline]
    pub fn is_into(&self) -> Option<bool> {
        self.into.is_true_opt()
    }

    #[inline]
    pub fn is_setter_into(&self) -> Option<bool> {
        self.setter.as_ref().and_then(|s| s.is_into())
    }

    #[inline]
    pub fn is_builder_into(&self) -> Option<bool> {
        self.builder.as_ref().and_then(|b| b.is_into())
    }

    #[inline]
    pub fn is_copy(&self) -> Option<bool> {
        if self.clone.is_true() {
            Some(false)
        }
        else {
            self.copy.is_true_opt()
        }
    }

    #[inline]
    pub fn is_clone(&self) -> Option<bool> {
        if self.copy.is_true() {
            Some(false)
        }
        else {
            self.clone.is_true_opt()
        }
    }

    #[inline]
    pub fn is_accessor_copy(&self) -> Option<bool> {
        self.accessor_mode().map(|m| m == FXAccessorMode::Copy)
    }

    #[inline]
    pub fn is_accessor_clone(&self) -> Option<bool> {
        self.accessor_mode().map(|m| m == FXAccessorMode::Clone)
    }

    #[inline]
    pub fn is_skipped(&self) -> bool {
        self.skip.is_present()
    }

    #[inline]
    pub fn is_inner_mut(&self) -> Option<bool> {
        self.inner_mut.is_true_opt()
    }

    #[inline]
    pub fn needs_lock(&self) -> Option<bool> {
        self.lock.as_ref().map(|l| l.is_true()).or_else(|| {
            self.is_sync().and_then(|s| {
                if s {
                    self.is_inner_mut()
                }
                else {
                    None
                }
            })
        })
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn is_serde(&self) -> Option<bool> {
        self.serde.as_ref().and_then(|sw| sw.is_serde())
    }

    #[inline]
    pub fn is_optional(&self) -> Option<bool> {
        self.optional.is_true_opt().or_else(|| {
            self.is_lazy().and_then(|l| {
                if l {
                    Some(false)
                }
                else {
                    None
                }
            })
        })
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn needs_serialize(&self) -> Option<bool> {
        self.serde.as_ref().and_then(|sw| sw.needs_serialize())
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn needs_deserialize(&self) -> Option<bool> {
        self.serde.as_ref().and_then(|sw| sw.needs_deserialize())
    }

    #[inline]
    pub fn has_default_value(&self) -> bool {
        if let Some(ref dv) = self.default_value {
            dv.is_true()
        }
        else {
            false
        }
    }

    pub fn accessor_mode(&self) -> Option<FXAccessorMode> {
        self.accessor.as_ref().and_then(|a| a.mode())
    }

    fn mark_implicitly(&mut self, orig: Meta) -> Result<(), &str> {
        match self.lazy {
            None => {
                self.lazy = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig.clone())));
                self.clearer = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig.clone())));
                self.predicate = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig)));
            }
            _ => (),
        };
        Ok(())
    }

    #[inline]
    pub fn set_span(&mut self, span: Span) -> Result<(), Span> {
        self.span.set(span)
    }

    #[inline]
    pub fn set_attr_span(&mut self, span: Span) {
        self.fieldx_attr_span = Some(span);
    }

    #[inline]
    pub fn span(&self) -> &Span {
        self.span.get_or_init(|| Span::call_site())
    }

    #[inline]
    pub fn accessor_mode_span(&self) -> Option<Span> {
        self.accessor
            .as_ref()
            .and_then(|a| a.mode_span())
            .or_else(|| {
                self.copy
                    .as_ref()
                    .and_then(|c| (c as &dyn fieldx_aux::FXOrig<_>).orig_span())
            })
            .or_else(|| {
                self.clone
                    .as_ref()
                    .and_then(|c| (c as &dyn fieldx_aux::FXOrig<_>).orig_span())
            })
    }

    #[inline]
    #[allow(dead_code)]
    pub fn serde_helper_span(&self) -> Option<Span> {
        self.serde.as_ref().and_then(|s| s.orig_span())
    }

    #[inline]
    pub fn fallible_error(&self) -> Option<&syn::Path> {
        self.fallible.as_ref().and_then(|f| f.error_type().map(|et| et.value()))
    }
}

impl FXHelperContainer for FXFieldReceiver {
    fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait> {
        match kind {
            FXHelperKind::Accessor => self.accessor().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::AccessorMut => self.accessor_mut().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Builder => self.builder().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Clearer => self.clearer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Lazy => self.lazy().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Predicate => self.predicate().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Reader => self.reader().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Setter => self.setter().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Writer => self.writer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
        }
    }

    fn get_helper_span(&self, kind: FXHelperKind) -> Option<Span> {
        match kind {
            FXHelperKind::Accessor => self.accessor().as_ref().map(|h| (h.span())),
            FXHelperKind::AccessorMut => self.accessor_mut().as_ref().map(|h| h.span()),
            FXHelperKind::Builder => self.builder().as_ref().map(|h| h.span()),
            FXHelperKind::Clearer => self.clearer().as_ref().map(|h| h.span()),
            FXHelperKind::Lazy => self.lazy().as_ref().map(|h| h.span()),
            FXHelperKind::Predicate => self.predicate().as_ref().map(|h| h.span()),
            FXHelperKind::Reader => self.reader().as_ref().map(|h| h.span()),
            FXHelperKind::Setter => self.setter().as_ref().map(|h| h.span()),
            FXHelperKind::Writer => self.writer().as_ref().map(|h| h.span()),
        }
    }
}
