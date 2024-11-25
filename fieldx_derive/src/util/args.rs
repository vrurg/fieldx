// #[cfg(feature = "diagnostics")]
// use crate::helper::FXOrig;
use crate::{
    helper::{FXHelperContainer, FXHelperKind},
    util::needs_helper,
};
use darling::FromMeta;
#[cfg(feature = "serde")]
use fieldx_aux::FXSerde;
use fieldx_aux::{
    validate_exclusives, FXAccessor, FXAccessorMode, FXAttributes, FXBoolArg, FXBoolHelper, FXBuilder, FXFallible,
    FXHelper, FXHelperTrait, FXNestingAttr, FXOrig, FXPubMode, FXSetter, FXSynValue, FXSyncMode, FXTriggerHelper,
};
use getset::Getters;
use proc_macro2::Span;

#[derive(Debug, FromMeta, Clone, Getters, Default)]
#[darling(and_then = Self::validate)]
#[getset(get = "pub")]
pub(crate) struct FXSArgs {
    #[getset(skip)]
    mode:       Option<FXSynValue<FXSyncMode>>,
    #[getset(skip)]
    #[darling(rename = "sync")]
    mode_sync:  Option<FXBoolArg>,
    #[getset(skip)]
    #[darling(rename = "r#async")]
    mode_async: Option<FXBoolArg>,

    builder: Option<FXBuilder>,
    into:    Option<FXBoolArg>,

    no_new:  Option<FXBoolArg>,
    default: Option<FXBoolArg>,
    // Produce reference counted object; i.e. Rc<Self> or Arc<Self>.
    rc:      Option<FXHelper>,

    attributes:      Option<FXAttributes>,
    attributes_impl: Option<FXAttributes>,

    // Field defaults
    fallible:     Option<FXNestingAttr<FXFallible>>,
    lazy:         Option<FXHelper>,
    #[darling(rename = "get")]
    accessor:     Option<FXAccessor>,
    #[darling(rename = "get_mut")]
    accessor_mut: Option<FXHelper>,
    #[darling(rename = "set")]
    setter:       Option<FXSetter>,
    reader:       Option<FXHelper>,
    writer:       Option<FXHelper>,
    clearer:      Option<FXHelper>,
    predicate:    Option<FXHelper>,
    optional:     Option<FXBoolArg>,
    public:       Option<FXNestingAttr<FXPubMode>>,
    private:      Option<FXBoolArg>,
    #[getset(get = "pub with_prefix")]
    clone:        Option<FXBoolArg>,
    #[getset(get = "pub with_prefix")]
    copy:         Option<FXBoolArg>,
    lock:         Option<FXBoolArg>,
    inner_mut:    Option<FXBoolArg>,
    #[cfg(feature = "serde")]
    serde:        Option<FXSerde>,
}

impl FXSArgs {
    #[cfg(feature = "serde")]
    validate_exclusives!(
        "visibility": public; private;
        "accessor mode": copy; clone;
        "concurrency mode": mode_sync as "sync", mode_async as "r#async"; inner_mut; mode;
        "field mode": lazy; optional, inner_mut;
        "serde/ref.counting": serde; rc;
    );

    #[cfg(not(feature = "serde"))]
    validate_exclusives!(
        "visibility": public; private;
        "accessor mode": copy, clone;
        "concurrency mode": mode_sync as "sync"; mode_async as "r#async"; inner_mut; mode;
        "field mode": lazy; optional, inner_mut;
    );

    // Generate needs_<helper> methods
    needs_helper! {accessor, accessor_mut, setter, reader, writer, clearer, predicate}

    #[inline]
    pub fn validate(self) -> Result<Self, darling::Error> {
        self.validate_exclusives()
            .map_err(|err| err.with_span(&Span::call_site()))?;
        Ok(self)
    }

    #[inline]
    pub fn is_sync(&self) -> Option<bool> {
        self.mode_sync
            .as_ref()
            .map(|th| th.is_true())
            .or_else(|| self.mode.as_ref().map(|m| m.is_sync() || m.is_async()))
            .or_else(|| self.is_async())
            .or_else(|| self.lock().as_ref().map(|th| th.is_true()))
            .or_else(|| self.reader().as_ref().map(|th| th.is_true()))
            .or_else(|| self.writer().as_ref().map(|th| th.is_true()))
    }

    #[inline]
    pub fn is_async(&self) -> Option<bool> {
        self.mode_async
            .as_ref()
            .map(|th| th.is_true())
            .or_else(|| self.mode.as_ref().map(|m| m.is_async()))
    }

    #[inline]
    pub fn is_fallible(&self) -> Option<bool> {
        self.fallible.is_true_opt()
    }

    #[inline]
    pub fn is_ref_counted(&self) -> bool {
        self.rc.is_true()
    }

    #[inline]
    pub fn is_into(&self) -> Option<bool> {
        self.into.is_true_opt()
    }

    #[inline]
    pub fn is_copy(&self) -> Option<bool> {
        if self.clone.is_true() {
            // Explicitly set `clone` means "not copy"
            Some(false)
        }
        else {
            self.copy.is_true_opt()
        }
    }

    #[inline]
    pub fn is_clone(&self) -> Option<bool> {
        if self.copy.is_true() {
            // Explicitly set `clone` means "not copy"
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
    pub fn is_setter_into(&self) -> Option<bool> {
        self.setter.as_ref().and_then(|h| h.is_into())
    }

    #[inline]
    pub fn is_builder_into(&self) -> Option<bool> {
        self.builder.as_ref().and_then(|h| h.is_into())
    }

    #[inline]
    pub fn is_builder_required(&self) -> Option<bool> {
        self.builder.as_ref().and_then(|h| h.is_required())
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn is_serde(&self) -> bool {
        self.serde.as_ref().map_or(false, |sw| sw.is_serde().unwrap_or(true))
    }

    #[inline]
    pub fn is_optional(&self) -> Option<bool> {
        self.optional.is_true_opt()
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn needs_serialize(&self) -> bool {
        self.serde
            .as_ref()
            .map_or(false, |sw| sw.needs_serialize().unwrap_or(true))
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn needs_deserialize(&self) -> bool {
        self.serde
            .as_ref()
            .map_or(false, |sw| sw.needs_deserialize().unwrap_or(true))
    }

    #[inline]
    pub fn needs_new(&self) -> bool {
        // new() is not possible without Default implementation
        self.no_new.not_true() && self.needs_default().unwrap_or(true)
    }

    #[inline]
    pub fn needs_default(&self) -> Option<bool> {
        self.default.is_true_opt()
    }

    #[inline]
    pub fn needs_builder(&self) -> Option<bool> {
        self.builder.is_true_opt()
    }

    #[inline]
    pub fn needs_lock(&self) -> Option<bool> {
        self.lock.as_ref().map(|b| b.is_true())
    }

    #[inline]
    pub fn is_lazy(&self) -> Option<bool> {
        self.lazy.is_true_opt()
    }

    #[inline]
    pub fn is_inner_mut(&self) -> Option<bool> {
        self.inner_mut.is_true_opt()
    }

    #[inline]
    pub fn accessor_mode(&self) -> Option<FXAccessorMode> {
        self.accessor.as_ref().and_then(|h| h.mode())
    }

    #[inline]
    pub fn builder_attributes(&self) -> Option<&FXAttributes> {
        self.builder
            .as_ref()
            .and_then(|b| b.attributes())
            .or_else(|| self.attributes().as_ref())
    }

    #[inline]
    pub fn builder_impl_attributes(&self) -> Option<&FXAttributes> {
        self.builder
            .as_ref()
            .and_then(|b| b.attributes_impl())
            .or_else(|| self.attributes_impl().as_ref())
    }

    #[inline(always)]
    pub fn builder_init_ident(&self) -> Option<syn::Ident> {
        self.builder
            .as_ref()
            .and_then(|b| b.init().as_ref().map(|v| v.value().clone()))
    }

    #[inline(always)]
    pub fn public_mode(&self) -> Option<FXPubMode> {
        fieldx_aux::public_mode(&self.public, &self.private)
    }

    #[inline]
    pub fn accessor_mode_span(&self) -> Option<Span> {
        self.accessor
            .as_ref()
            .and_then(|a| a.mode_span())
            .or_else(|| {
                self.copy
                    .as_ref()
                    .and_then(|c| (c as &dyn fieldx_aux::FXOrig<_>).span())
            })
            .or_else(|| {
                self.clone
                    .as_ref()
                    .and_then(|c| (c as &dyn fieldx_aux::FXOrig<_>).span())
            })
    }

    #[inline]
    pub fn fallible_error(&self) -> Option<&syn::Path> {
        self.fallible.as_ref().and_then(|f| f.error_type().map(|et| et.value()))
    }
}

impl FXHelperContainer for FXSArgs {
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
            FXHelperKind::Accessor => self.accessor().as_ref().and_then(|h| h.span()),
            FXHelperKind::AccessorMut => self.accessor_mut().as_ref().and_then(|h| h.span()),
            FXHelperKind::Builder => self.builder().as_ref().and_then(|h| h.span()),
            FXHelperKind::Clearer => self.clearer().as_ref().and_then(|h| h.span()),
            FXHelperKind::Lazy => self.lazy().as_ref().and_then(|h| h.span()),
            FXHelperKind::Predicate => self.predicate().as_ref().and_then(|h| h.span()),
            FXHelperKind::Reader => self.reader().as_ref().and_then(|h| h.span()),
            FXHelperKind::Setter => self.setter().as_ref().and_then(|h| h.span()),
            FXHelperKind::Writer => self.writer().as_ref().and_then(|h| h.span()),
        }
    }
}
