pub(crate) mod props;

use darling::util::Flag;
use darling::FromField;
use darling::FromMeta;
use fieldx_aux::validate_exclusives;
use fieldx_aux::validate_no_macro_args;
use fieldx_aux::FXAccessor;
use fieldx_aux::FXAttributes;
use fieldx_aux::FXBool;
use fieldx_aux::FXBuilder;
use fieldx_aux::FXDefault;
use fieldx_aux::FXFallible;
use fieldx_aux::FXHelper;
use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
#[cfg(feature = "serde")]
use fieldx_aux::FXSerde;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSetter;
use fieldx_aux::FXString;
use fieldx_aux::FXSynValue;
use fieldx_aux::FXSyncMode;
use getset::Getters;
use once_cell::unsync::OnceCell;
use proc_macro2::Span;
use proc_macro2::TokenStream;
pub(crate) use props::FXFieldProps;
use quote::quote_spanned;
use quote::ToTokens;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::Meta;

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

    #[darling(rename = "vis")]
    visibility:    Option<FXSynValue<syn::Visibility>>,
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
    #[cfg(feature = "serde")]
    serde:         Option<FXSerde>,

    #[darling(skip)]
    #[getset(skip)]
    span: OnceCell<Span>,

    #[darling(skip)]
    fieldx_attr_span: Option<Span>,

    #[darling(skip)]
    #[getset(skip)]
    extra: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FXField(FXFieldReceiver);

impl FXField {
    #[inline]
    pub(crate) fn extra(mut self) -> Self {
        self.0.extra = true;
        self
    }

    #[inline(always)]
    pub(crate) fn is_extra(&self) -> bool {
        self.0.extra
    }
}

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
        "accessor mode": copy; clone;
        "field mode":  lazy; optional;
        "in-/fallible mode": fallible; lock, optional, inner_mut;
        "concurrency mode": mode_sync as "sync"; mode_async as "async"; mode;
        "visibility": private; visibility as "vis";
    }

    // Generate field-level needs_<helper> methods. The final decision of what's needed and what's not is done by
    // FXFieldCtx.
    // needs_helper! {accessor, accessor_mut, builder, clearer, setter, predicate, reader, writer}

    pub(crate) fn validate(&self) -> darling::Result<()> {
        let mut acc = darling::Error::accumulator();

        if let Err(err) = self.validate_exclusives() {
            acc.push(err);
        }

        #[cfg(feature = "serde")]
        validate_no_macro_args! {
            "field", self, acc:
                serde.shadow_name,
                serde.orig_visibility as visibility,
                serde.private,
        }

        validate_no_macro_args! {
            "field", self, acc:
                builder.prefix,
                builder.method_doc,
                builder.attributes_impl,
                builder.post_build,
                builder.opt_in,
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

        // #[cfg(not(feature = "serde"))]
        // if let Some(err) = crate::util::feature_required("serde", &self.serde) {
        //     acc.push(err);
        // }

        acc.finish()?;

        Ok(())
    }

    pub(crate) fn ident(&self) -> darling::Result<syn::Ident> {
        self.ident.clone().ok_or_else(|| {
            darling::Error::custom("This is weird, but the field doesn't have an ident!").with_span(self.span())
        })
    }

    #[inline]
    pub(crate) fn has_default_value(&self) -> bool {
        if let Some(ref dv) = self.default_value {
            *dv.is_set()
        }
        else {
            false
        }
    }

    fn mark_implicitly(&mut self, orig: Meta) -> darling::Result<()> {
        match self.lazy {
            None => {
                // self.lazy = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig.clone())));
                self.lazy = Some(FXNestingAttr::from_meta(&syn::parse2::<syn::Meta>(
                    quote_spanned! {orig.span()=> lazy},
                )?)?);
                self.clearer = Some(FXNestingAttr::from_meta(&syn::parse2::<syn::Meta>(
                    quote_spanned! {orig.span()=> clearer},
                )?)?);
                self.predicate = Some(FXNestingAttr::from_meta(&syn::parse2::<syn::Meta>(
                    quote_spanned! {orig.span()=> predicate},
                )?)?);
            }
            _ => (),
        };
        Ok(())
    }

    #[inline]
    pub(crate) fn set_span(&mut self, span: Span) -> Result<(), Span> {
        self.span.set(span)
    }

    #[inline]
    pub(crate) fn set_attr_span(&mut self, span: Span) {
        self.fieldx_attr_span = Some(span);
    }

    #[inline]
    pub(crate) fn span(&self) -> &Span {
        self.span.get_or_init(|| Span::call_site())
    }
}

// impl FXHelperContainer for FXFieldReceiver {
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
//             FXHelperKind::Accessor => self.accessor().as_ref().map(|h| (h.span())),
//             FXHelperKind::AccessorMut => self.accessor_mut().as_ref().map(|h| h.span()),
//             FXHelperKind::Builder => self.builder().as_ref().map(|h| h.span()),
//             FXHelperKind::Clearer => self.clearer().as_ref().map(|h| h.span()),
//             FXHelperKind::Lazy => self.lazy().as_ref().map(|h| h.span()),
//             FXHelperKind::Predicate => self.predicate().as_ref().map(|h| h.span()),
//             FXHelperKind::Reader => self.reader().as_ref().map(|h| h.span()),
//             FXHelperKind::Setter => self.setter().as_ref().map(|h| h.span()),
//             FXHelperKind::Writer => self.writer().as_ref().map(|h| h.span()),
//         }
//     }
// }
