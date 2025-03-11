use crate::{
    helper::FXHelperKind,
    util::{common_prop_impl, mode_async_prop, mode_plain_prop, mode_sync_prop, simple_bool_prop, simple_type_prop},
};
use darling::{util::Flag, FromField};
use fieldx_aux::{
    validate_exclusives, FXAccessor, FXAccessorMode, FXAttributes, FXBaseHelper, FXBool, FXBoolHelper, FXBuilder,
    FXDefault, FXFallible, FXHelper, FXHelperTrait, FXNestingAttr, FXOrig, FXProp, FXSerde, FXSetState, FXSetter,
    FXString, FXSynValue, FXSyncMode, FXTriggerHelper,
};
use getset::Getters;
use once_cell::unsync::OnceCell;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use std::ops::Deref;
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
        "accessor mode": copy; clone;
        "field mode":  lazy; optional;
        "in-/fallible mode": fallible; lock, optional, inner_mut;
        "concurrency mode": mode_sync as "sync"; mode_async as "async"; mode;
        "visibility": private; visibility as "vis";
    }

    // Generate field-level needs_<helper> methods. The final decision of what's needed and what's not is done by
    // FXFieldCtx.
    // needs_helper! {accessor, accessor_mut, builder, clearer, setter, predicate, reader, writer}

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

    pub fn ident(&self) -> darling::Result<syn::Ident> {
        self.ident.clone().ok_or_else(|| {
            darling::Error::custom("This is weird, but the field doesn't have an ident!").with_span(self.span())
        })
    }

    #[inline]
    pub fn has_default_value(&self) -> bool {
        if let Some(ref dv) = self.default_value {
            *dv.is_true()
        }
        else {
            false
        }
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

#[derive(Debug)]
pub struct FXFieldProps {
    source: FXField,

    // --- Helper properties
    // Accessor helper standard properties
    accessor:                OnceCell<Option<FXProp<bool>>>,
    accessor_visibility:     OnceCell<Option<syn::Visibility>>,
    accessor_ident:          OnceCell<Option<syn::Ident>>,
    // Accessor helper extended properties
    accessor_mode:           OnceCell<Option<FXProp<FXAccessorMode>>>,
    // Mutable accessor helper standard properties
    accessor_mut:            OnceCell<Option<FXProp<bool>>>,
    accessor_mut_visibility: OnceCell<Option<syn::Visibility>>,
    accessor_mut_ident:      OnceCell<Option<syn::Ident>>,
    // Builder helper standard properties
    builder:                 OnceCell<Option<FXProp<bool>>>,
    builder_visibility:      OnceCell<Option<syn::Visibility>>,
    builder_ident:           OnceCell<Option<syn::Ident>>,
    // Builder helper extended properties
    builder_into:            OnceCell<Option<FXProp<bool>>>,
    builder_required:        OnceCell<Option<FXProp<bool>>>,
    // Corresponding builder field attributes
    builder_attributes:      OnceCell<Option<FXAttributes>>,
    // Clearer helper standard properties
    clearer:                 OnceCell<Option<FXProp<bool>>>,
    clearer_visibility:      OnceCell<Option<syn::Visibility>>,
    clearer_ident:           OnceCell<Option<syn::Ident>>,
    // Predicate helper standard properties
    predicate:               OnceCell<Option<FXProp<bool>>>,
    predicate_visibility:    OnceCell<Option<syn::Visibility>>,
    predicate_ident:         OnceCell<Option<syn::Ident>>,
    // Reader helper standard properties
    reader:                  OnceCell<Option<FXProp<bool>>>,
    reader_visibility:       OnceCell<Option<syn::Visibility>>,
    reader_ident:            OnceCell<Option<syn::Ident>>,
    // Setter helper standard properties
    setter:                  OnceCell<Option<FXProp<bool>>>,
    setter_visibility:       OnceCell<Option<syn::Visibility>>,
    setter_ident:            OnceCell<Option<syn::Ident>>,
    // Setter helper extended properties
    setter_into:             OnceCell<Option<FXProp<bool>>>,
    // Writer helper standard properties
    writer:                  OnceCell<Option<FXProp<bool>>>,
    writer_visibility:       OnceCell<Option<syn::Visibility>>,
    writer_ident:            OnceCell<Option<syn::Ident>>,
    // Lazy helper standard properties
    lazy:                    OnceCell<Option<FXProp<bool>>>,
    lazy_visibility:         OnceCell<Option<syn::Visibility>>,
    lazy_ident:              OnceCell<Option<syn::Ident>>,
    // --- Other properties
    // Base name of the field. Normally would be the same as the field name.
    base_name:               OnceCell<Option<syn::Ident>>,
    fallible:                OnceCell<Option<FXProp<FXFallible>>>,
    inner_mut:               OnceCell<Option<FXProp<bool>>>,
    into:                    OnceCell<Option<FXProp<bool>>>,
    lock:                    OnceCell<Option<FXProp<bool>>>,
    mode_async:              OnceCell<Option<FXProp<bool>>>,
    mode_plain:              OnceCell<Option<FXProp<bool>>>,
    mode_sync:               OnceCell<Option<FXProp<bool>>>,
    optional:                OnceCell<Option<FXProp<bool>>>,
    skipped:                 OnceCell<FXProp<bool>>,
    syncish:                 OnceCell<FXProp<bool>>,
    visibility:              OnceCell<Option<syn::Visibility>>,
    default_value:           OnceCell<Option<syn::Expr>>,
    has_default:             OnceCell<FXProp<bool>>,

    #[cfg(feature = "serde")]
    serde:                    OnceCell<Option<FXProp<Option<bool>>>>,
    #[cfg(feature = "serde")]
    serialize:                OnceCell<Option<FXProp<bool>>>,
    #[cfg(feature = "serde")]
    deserialize:              OnceCell<Option<FXProp<bool>>>,
    #[cfg(feature = "serde")]
    serde_default_value:      OnceCell<Option<FXDefault>>,
    #[cfg(feature = "serde")]
    serde_rename_serialize:   OnceCell<Option<FXProp<String>>>,
    #[cfg(feature = "serde")]
    serde_rename_deserialize: OnceCell<Option<FXProp<String>>>,
}

impl FXFieldProps {
    common_prop_impl! {}

    simple_type_prop! {
        fallible, FXFallible;
    }

    pub fn new(field: FXField) -> Self {
        Self {
            source: field,

            accessor:                OnceCell::new(),
            accessor_visibility:     OnceCell::new(),
            accessor_ident:          OnceCell::new(),
            accessor_mode:           OnceCell::new(),
            accessor_mut:            OnceCell::new(),
            accessor_mut_visibility: OnceCell::new(),
            accessor_mut_ident:      OnceCell::new(),
            builder:                 OnceCell::new(),
            builder_attributes:      OnceCell::new(),
            builder_visibility:      OnceCell::new(),
            builder_ident:           OnceCell::new(),
            builder_into:            OnceCell::new(),
            builder_required:        OnceCell::new(),
            clearer:                 OnceCell::new(),
            clearer_visibility:      OnceCell::new(),
            clearer_ident:           OnceCell::new(),
            predicate:               OnceCell::new(),
            predicate_visibility:    OnceCell::new(),
            predicate_ident:         OnceCell::new(),
            reader:                  OnceCell::new(),
            reader_visibility:       OnceCell::new(),
            reader_ident:            OnceCell::new(),
            setter:                  OnceCell::new(),
            setter_visibility:       OnceCell::new(),
            setter_ident:            OnceCell::new(),
            setter_into:             OnceCell::new(),
            writer:                  OnceCell::new(),
            writer_visibility:       OnceCell::new(),
            writer_ident:            OnceCell::new(),
            lazy:                    OnceCell::new(),
            lazy_visibility:         OnceCell::new(),
            lazy_ident:              OnceCell::new(),
            base_name:               OnceCell::new(),
            fallible:                OnceCell::new(),
            inner_mut:               OnceCell::new(),
            into:                    OnceCell::new(),
            lock:                    OnceCell::new(),
            mode_async:              OnceCell::new(),
            mode_plain:              OnceCell::new(),
            mode_sync:               OnceCell::new(),
            optional:                OnceCell::new(),
            skipped:                 OnceCell::new(),
            syncish:                 OnceCell::new(),
            visibility:              OnceCell::new(),
            default_value:           OnceCell::new(),
            has_default:             OnceCell::new(),

            #[cfg(feature = "serde")]
            serde:                                              OnceCell::new(),
            #[cfg(feature = "serde")]
            serialize:                                          OnceCell::new(),
            #[cfg(feature = "serde")]
            deserialize:                                        OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_default_value:                                OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_rename_serialize:                             OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_rename_deserialize:                           OnceCell::new(),
        }
    }

    #[inline(always)]
    pub fn field(&self) -> &FXField {
        &self.source
    }

    // Returns a true FXProp only if either `lock`, `writer`, or `reader` is set.
    // Otherwise, returns `None`.
    pub fn syncish(&self) -> FXProp<bool> {
        *self.syncish.get_or_init(|| {
            self.source
                .lock()
                .as_ref()
                .and_then(|l| {
                    if *l.is_true() {
                        Some(l.into())
                    }
                    else {
                        None
                    }
                })
                .or_else(|| {
                    self.source.reader().as_ref().and_then(|r| {
                        if *r.is_true() {
                            Some(r.into())
                        }
                        else {
                            None
                        }
                    })
                })
                .or_else(|| {
                    self.source.writer().as_ref().and_then(|w| {
                        if *w.is_true() {
                            Some(w.into())
                        }
                        else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| FXProp::new(false, None))
        })
    }

    pub fn skipped(&self) -> FXProp<bool> {
        *self.skipped.get_or_init(|| self.source.skip().into())
    }

    pub fn visibility(&self) -> Option<&syn::Visibility> {
        self.visibility
            .get_or_init(|| {
                if *self.source.private.is_true() {
                    return Some(syn::Visibility::Inherited);
                }
                self.source.visibility.as_ref().map(|v| v.value().clone())
            })
            .as_ref()
    }

    pub fn builder_attributes(&self) -> Option<&FXAttributes> {
        self.builder_attributes
            .get_or_init(|| self.source.builder().as_ref().and_then(|b| b.attributes()).cloned())
            .as_ref()
    }

    pub fn base_name(&self) -> Option<&syn::Ident> {
        self.base_name
            .get_or_init(|| {
                if let Some(ref bn) = self.source.base_name {
                    bn.value().map(|name| syn::Ident::new(name, bn.span()))
                }
                else {
                    None
                }
            })
            .as_ref()
    }

    pub fn default_value(&self) -> Option<&syn::Expr> {
        self.default_value
            .get_or_init(|| {
                self.source
                    .default_value()
                    .as_ref()
                    .and_then(|d| {
                        if *d.is_set() {
                            d.value()
                        }
                        else {
                            None
                        }
                    })
                    .cloned()
            })
            .as_ref()
    }

    pub fn has_default(&self) -> FXProp<bool> {
        *self.has_default.get_or_init(|| {
            self.source
                .default_value()
                .as_ref()
                .map_or_else(|| false.into(), |d| d.is_true())
        })
    }

    #[cfg(feature = "serde")]
    pub fn serde(&self) -> Option<FXProp<Option<bool>>> {
        *self.serde.get_or_init(|| {
            self.source
                .serde
                .as_ref()
                .and_then(|s| Some(s.is_serde().respan(s.orig_span())))
        })
    }

    #[cfg(feature = "serde")]
    pub fn serialize(&self) -> Option<FXProp<bool>> {
        *self
            .serialize
            .get_or_init(|| self.source.serde().as_ref().and_then(|s| s.needs_serialize()))
    }

    #[cfg(feature = "serde")]
    pub fn deserialize(&self) -> Option<FXProp<bool>> {
        *self
            .deserialize
            .get_or_init(|| self.source.serde.as_ref().and_then(|s| s.needs_deserialize()))
    }
}
