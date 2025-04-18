pub(crate) mod props;

use darling::FromMeta;
use fieldx_aux::validate_exclusives;
use fieldx_aux::validate_no_macro_args;
use fieldx_aux::FXAccessor;
use fieldx_aux::FXAttributes;
use fieldx_aux::FXBool;
use fieldx_aux::FXBuilder;
use fieldx_aux::FXFallible;
use fieldx_aux::FXHelper;
use fieldx_aux::FXHelperTrait;
use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
use fieldx_aux::FXSerde;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSetter;
use fieldx_aux::FXSynValue;
use fieldx_aux::FXSyncMode;
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
    new:     Option<FXHelper>,
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

    #[inline]
    pub(crate) fn validate(self) -> Result<Self, darling::Error> {
        let mut acc = darling::Error::accumulator();

        if let Err(err) = self.validate_exclusives() {
            acc.push(err);
        }

        validate_no_macro_args! {
            "struct", self, acc:
                accessor as get.doc,
                accessor_mut as get_mut.doc,
                clearer.doc,
                predicate.doc,
                reader.doc,
                setter.doc,
                writer.doc,
                lazy.doc
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
}
