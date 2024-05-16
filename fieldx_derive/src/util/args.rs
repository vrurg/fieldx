// #[cfg(feature = "diagnostics")]
// use crate::helper::FXOrig;
#[cfg(feature = "serde")]
use crate::helper::FXSerde;
use crate::{
    helper::{
        FXAccessor, FXAccessorMode, FXArgsBuilder, FXAttributes, FXBoolArg, FXHelper, FXHelperContainer, FXHelperKind,
        FXHelperTrait, FXNestingAttr, FXPubMode, FXSetter, FXTriggerHelper,
    },
    util::{needs_helper, validate_exclusives},
};
use darling::{util::Flag, FromMeta};
use getset::Getters;
use proc_macro2::Span;

#[derive(Debug, FromMeta, Clone, Getters, Default)]
#[darling(and_then = Self::validate)]
#[getset(get = "pub")]
pub(crate) struct FXSArgs {
    sync:    Flag,
    builder: Option<FXArgsBuilder>,
    into:    Option<bool>,
    // Only plays for sync-safe structs
    no_new:  Flag,

    // Field defaults
    lazy:         Option<FXHelper<true>>,
    #[darling(rename = "get")]
    accessor:     Option<FXAccessor<true>>,
    #[darling(rename = "get_mut")]
    accessor_mut: Option<FXHelper<true>>,
    #[darling(rename = "set")]
    setter:       Option<FXSetter<true>>,
    reader:       Option<FXHelper<true>>,
    writer:       Option<FXHelper<true>>,
    clearer:      Option<FXHelper<true>>,
    predicate:    Option<FXHelper<true>>,
    public:       Option<FXNestingAttr<FXPubMode>>,
    private:      Option<FXBoolArg>,
    clone:        Option<FXBoolArg>,
    copy:         Option<FXBoolArg>,
    #[cfg(feature = "serde")]
    serde:        Option<FXSerde>,
}

impl FXSArgs {
    validate_exclusives!("visibility" => public, private; "accessor mode" => copy, clone);

    // Generate needs_<helper> methods
    needs_helper! {accessor, accessor_mut, setter, reader, writer, clearer, predicate}

    pub fn validate(self) -> Result<Self, darling::Error> {
        self.validate_exclusives()
            .map_err(|err| err.with_span(&Span::call_site()))?;
        Ok(self)
    }

    pub fn is_sync(&self) -> bool {
        self.sync.is_present()
    }

    pub fn is_into(&self) -> Option<bool> {
        self.into
    }

    pub fn is_copy(&self) -> Option<bool> {
        self.clone
            .as_ref()
            .map(|c| !c.is_true())
            .or_else(|| self.copy.as_ref().map(|c| c.is_true()))
    }

    pub fn is_accessor_copy(&self) -> Option<bool> {
        self.accessor_mode().map(|m| m == FXAccessorMode::Copy)
    }

    pub fn is_setter_into(&self) -> Option<bool> {
        self.setter.as_ref().and_then(|h| h.is_into())
    }

    pub fn is_builder_into(&self) -> Option<bool> {
        self.builder.as_ref().and_then(|h| h.is_into())
    }

    #[cfg(feature = "serde")]
    pub fn is_serde(&self) -> bool {
        self.serde.as_ref().map_or(false, |sw| sw.is_serde().unwrap_or(true))
    }

    #[cfg(feature = "serde")]
    pub fn needs_serialize(&self) -> bool {
        self.serde
            .as_ref()
            .map_or(false, |sw| sw.needs_serialize().unwrap_or(true))
    }

    #[cfg(feature = "serde")]
    pub fn needs_deserialize(&self) -> bool {
        self.serde
            .as_ref()
            .map_or(false, |sw| sw.needs_deserialize().unwrap_or(true))
    }

    pub fn needs_new(&self) -> bool {
        !self.no_new.is_present()
    }

    pub fn needs_builder(&self) -> Option<bool> {
        self.builder.as_ref().and(Some(true))
    }

    pub fn is_lazy(&self) -> Option<bool> {
        self.lazy.as_ref().map(|h| h.is_true())
    }

    pub fn accessor_mode(&self) -> Option<FXAccessorMode> {
        self.accessor.as_ref().and_then(|h| h.mode())
    }

    pub fn builder_attributes(&self) -> Option<&FXAttributes> {
        self.builder.as_ref().and_then(|b| b.attributes().as_ref())
    }

    pub fn builder_impl_attributes(&self) -> Option<&FXAttributes> {
        self.builder.as_ref().and_then(|b| b.attributes_impl().as_ref())
    }

    pub fn public_mode(&self) -> Option<FXPubMode> {
        if self.private.as_ref().map_or(false, |p| p.is_true()) {
            Some(FXPubMode::Private)
        }
        else {
            self.public.as_ref().map(|pm| (**pm).clone())
        }
    }
}

impl FXHelperContainer for FXSArgs {
    fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait> {
        match kind {
            // FXHelperKind::Lazy => self.lazy().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Accessor => self.accessor().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::AccesorMut => self.accessor_mut().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Clearer => self.clearer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Predicate => self.predicate().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Reader => self.reader().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Setter => self.setter().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Writer => self.writer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
        }
    }
}
