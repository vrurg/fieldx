pub(crate) mod props;

use darling::FromMeta;
use fieldx_aux::{
    validate_exclusives, validate_no_macro_args, FXAccessor, FXAttributes, FXBool, FXBuilder, FXFallible, FXHelper,
    FXNestingAttr, FXSerde, FXSetState, FXSetter, FXSynValue, FXSyncMode,
};
use getset::Getters;
pub(crate) use props::FXArgProps;

#[derive(Debug, FromMeta, Clone, Getters, Default)]
#[darling(and_then = Self::validate)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXSArgs {
    #[getset(skip)]
    mode:       Option<FXSynValue<FXSyncMode>>,
    #[getset(skip)]
    #[darling(rename = "sync")]
    mode_sync:  Option<FXBool>,
    #[getset(skip)]
    #[darling(rename = "r#async")]
    mode_async: Option<FXBool>,

    builder: Option<FXBuilder<true>>,
    into:    Option<FXBool>,

    no_new:  Option<FXBool>,
    default: Option<FXBool>,
    // Produce reference counted object; i.e. Rc<Self> or Arc<Self>.
    rc:      Option<FXHelper>,

    attributes:      Option<FXAttributes>,
    attributes_fn:   Option<FXAttributes>,
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
    optional:     Option<FXBool>,
    #[darling(rename = "vis")]
    visibility:   Option<FXSynValue<syn::Visibility>>,
    private:      Option<FXBool>,
    #[getset(get = "pub(crate) with_prefix")]
    clone:        Option<FXBool>,
    #[getset(get = "pub(crate) with_prefix")]
    copy:         Option<FXBool>,
    lock:         Option<FXBool>,
    inner_mut:    Option<FXBool>,
    serde:        Option<FXSerde>,
    // #[cfg(not(feature = "serde"))]
    // serde:        Option<fieldx_aux::syn_value::FXPunctuated<syn::Meta, syn::Token![,]>>,
}

impl FXSArgs {
    validate_exclusives!(
        "accessor mode": copy; clone;
        "concurrency mode": mode_sync as "sync", mode_async as "r#async"; mode;
        "field mode": lazy; optional;
        "serde/ref.counting": serde; rc;
        "visibility": private; visibility as "vis";
    );

    validate_no_macro_args! {
        "struct":
            accessor as get.doc, accessor_mut as get_mut.doc, clearer.doc, predicate.doc, reader.doc, setter.doc,
            writer.doc, lazy.doc
    }

    #[inline]
    pub(crate) fn validate(self) -> Result<Self, darling::Error> {
        let mut acc = darling::Error::accumulator();

        if let Err(err) = self.validate_exclusives() {
            acc.push(err);
        }

        if let Err(err) = self.validate_subargs() {
            acc.push(err);
        }

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

        Ok(self)
    }

    // #[inline]
    // pub(crate) fn is_ref_counted(&self) -> bool {
    //     self.rc.is_true()
    // }

    // #[inline]
    // pub(crate) fn is_builder_into(&self) -> Option<bool> {
    //     self.builder.as_ref().and_then(|h| h.is_into())
    // }

    // #[inline]
    // pub(crate) fn is_builder_required(&self) -> Option<bool> {
    //     self.builder.as_ref().and_then(|h| h.is_required())
    // }

    // #[inline]
    // pub(crate) fn needs_new(&self) -> bool {
    //     // new() is not possible without Default implementation
    //     self.no_new.not_true() && self.needs_default().unwrap_or(true)
    // }

    // #[inline]
    // pub(crate) fn needs_default(&self) -> Option<bool> {
    //     self.default.is_true_opt()
    // }

    // #[inline]
    // pub(crate) fn accessor_mode(&self) -> Option<FXAccessorMode> {
    //     self.accessor.as_ref().and_then(|h| h.mode())
    // }

    // #[inline]
    // pub(crate) fn builder_attributes(&self) -> Option<&FXAttributes> {
    //     self.builder
    //         .as_ref()
    //         .and_then(|b| b.attributes())
    //         .or_else(|| self.attributes().as_ref())
    // }

    // #[inline]
    // pub(crate) fn builder_impl_attributes(&self) -> Option<&FXAttributes> {
    //     self.builder
    //         .as_ref()
    //         .and_then(|b| b.attributes_impl())
    //         .or_else(|| self.attributes_impl().as_ref())
    // }

    // #[inline]
    // pub(crate) fn accessor_mode_span(&self) -> Option<Span> {
    //     self.accessor
    //         .as_ref()
    //         .and_then(|a| a.mode_span())
    //         .or_else(|| {
    //             self.copy
    //                 .as_ref()
    //                 .and_then(|c| (c as &dyn fieldx_aux::FXOrig<_>).orig_span())
    //         })
    //         .or_else(|| {
    //             self.clone
    //                 .as_ref()
    //                 .and_then(|c| (c as &dyn fieldx_aux::FXOrig<_>).orig_span())
    //         })
    // }
}

// impl FXHelperContainer for FXSArgs {
//     fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait> {
//         match kind {
//             FXHelperKind::Accessor => self.accessor().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::AccessorMut => self.accessor_mut().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Builder => self.builder().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Clearer => self.clearer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Lazy => self.lazy().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Predicate => self.predicate().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Reader => self.reader().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Setter => self.setter().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//             FXHelperKind::Writer => self.writer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
//         }
//     }

//     fn get_helper_span(&self, kind: FXHelperKind) -> Option<Span> {
//         match kind {
//             FXHelperKind::Accessor => self.accessor().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::AccessorMut => self.accessor_mut().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Builder => self.builder().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Clearer => self.clearer().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Lazy => self.lazy().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Predicate => self.predicate().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Reader => self.reader().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Setter => self.setter().as_ref().and_then(|h| h.orig_span()),
//             FXHelperKind::Writer => self.writer().as_ref().and_then(|h| h.orig_span()),
//         }
//     }
// }
