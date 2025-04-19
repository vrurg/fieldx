use crate::helper::FXHelperKind;
use crate::util::common_prop_impl;
use crate::util::doc_props;
use crate::util::mode_async_prop;
use crate::util::mode_plain_prop;
use crate::util::mode_sync_prop;
use crate::util::simple_bool_prop;
use crate::util::simple_type_prop;
use fieldx_aux::FXAccessorMode;
use fieldx_aux::FXAttributes;
use fieldx_aux::FXBoolHelper;
#[cfg(feature = "serde")]
use fieldx_aux::FXDefault;
use fieldx_aux::FXFallible;
use fieldx_aux::FXHelperTrait;
use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
use fieldx_aux::FXProp;
use fieldx_aux::FXSetState;
use once_cell::unsync::OnceCell;

use super::FXField;

#[derive(Debug)]
pub(crate) struct FXFieldProps {
    source: FXField,

    // --- Helper properties
    // Accessor helper standard properties
    accessor:                OnceCell<Option<FXProp<bool>>>,
    accessor_visibility:     OnceCell<Option<syn::Visibility>>,
    accessor_ident:          OnceCell<Option<syn::Ident>>,
    // Accessor helper extended properties
    accessor_mode:           OnceCell<Option<FXProp<FXAccessorMode>>>,
    accessor_doc:            OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Mutable accessor helper standard properties
    accessor_mut:            OnceCell<Option<FXProp<bool>>>,
    accessor_mut_visibility: OnceCell<Option<syn::Visibility>>,
    accessor_mut_ident:      OnceCell<Option<syn::Ident>>,
    // Mutable accessor helper extended properties
    accessor_mut_doc:        OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Builder helper standard properties
    builder:                 OnceCell<Option<FXProp<bool>>>,
    builder_visibility:      OnceCell<Option<syn::Visibility>>,
    builder_ident:           OnceCell<Option<syn::Ident>>,
    // Builder helper extended properties
    builder_into:            OnceCell<Option<FXProp<bool>>>,
    builder_required:        OnceCell<Option<FXProp<bool>>>,
    builder_doc:             OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Corresponding builder field attributes
    builder_attributes:      OnceCell<Option<FXAttributes>>,
    // Clearer helper standard properties
    clearer:                 OnceCell<Option<FXProp<bool>>>,
    clearer_visibility:      OnceCell<Option<syn::Visibility>>,
    clearer_ident:           OnceCell<Option<syn::Ident>>,
    // Clearer helper extended properties
    clearer_doc:             OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Predicate helper standard properties
    predicate:               OnceCell<Option<FXProp<bool>>>,
    predicate_visibility:    OnceCell<Option<syn::Visibility>>,
    predicate_ident:         OnceCell<Option<syn::Ident>>,
    // Predicate helper extended properties
    predicate_doc:           OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Reader helper standard properties
    reader:                  OnceCell<Option<FXProp<bool>>>,
    reader_visibility:       OnceCell<Option<syn::Visibility>>,
    reader_ident:            OnceCell<Option<syn::Ident>>,
    // Reader helper extended properties
    reader_doc:              OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Setter helper standard properties
    setter:                  OnceCell<Option<FXProp<bool>>>,
    setter_visibility:       OnceCell<Option<syn::Visibility>>,
    setter_ident:            OnceCell<Option<syn::Ident>>,
    // Setter helper extended properties
    setter_into:             OnceCell<Option<FXProp<bool>>>,
    setter_doc:              OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Writer helper standard properties
    writer:                  OnceCell<Option<FXProp<bool>>>,
    writer_visibility:       OnceCell<Option<syn::Visibility>>,
    writer_ident:            OnceCell<Option<syn::Ident>>,
    // Writer helper extended properties
    writer_doc:              OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
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
    doc:                     OnceCell<Vec<syn::Attribute>>,

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
    common_prop_impl! {
        accessor, accessor_mut, builder, setter, clearer, predicate, reader, writer, lazy
    }

    doc_props! {
        accessor_doc from accessor.doc;
        accessor_mut_doc from accessor_mut.doc;
        builder_doc from builder.doc;
        clearer_doc from clearer.doc;
        predicate_doc from predicate.doc;
        reader_doc from reader.doc;
        setter_doc from setter.doc;
        writer_doc from writer.doc;
    }

    simple_type_prop! {
        fallible, FXFallible;
    }

    pub(crate) fn new(field: FXField) -> Self {
        Self {
            source: field,

            accessor:                OnceCell::new(),
            accessor_visibility:     OnceCell::new(),
            accessor_ident:          OnceCell::new(),
            accessor_mode:           OnceCell::new(),
            accessor_doc:            OnceCell::new(),
            accessor_mut:            OnceCell::new(),
            accessor_mut_visibility: OnceCell::new(),
            accessor_mut_ident:      OnceCell::new(),
            accessor_mut_doc:        OnceCell::new(),
            builder:                 OnceCell::new(),
            builder_attributes:      OnceCell::new(),
            builder_visibility:      OnceCell::new(),
            builder_ident:           OnceCell::new(),
            builder_into:            OnceCell::new(),
            builder_required:        OnceCell::new(),
            builder_doc:             OnceCell::new(),
            clearer:                 OnceCell::new(),
            clearer_visibility:      OnceCell::new(),
            clearer_ident:           OnceCell::new(),
            clearer_doc:             OnceCell::new(),
            predicate:               OnceCell::new(),
            predicate_visibility:    OnceCell::new(),
            predicate_ident:         OnceCell::new(),
            predicate_doc:           OnceCell::new(),
            reader:                  OnceCell::new(),
            reader_visibility:       OnceCell::new(),
            reader_ident:            OnceCell::new(),
            reader_doc:              OnceCell::new(),
            setter:                  OnceCell::new(),
            setter_visibility:       OnceCell::new(),
            setter_ident:            OnceCell::new(),
            setter_into:             OnceCell::new(),
            setter_doc:              OnceCell::new(),
            writer:                  OnceCell::new(),
            writer_visibility:       OnceCell::new(),
            writer_ident:            OnceCell::new(),
            writer_doc:              OnceCell::new(),
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
            doc:                     OnceCell::new(),

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
    pub(crate) fn field(&self) -> &FXField {
        &self.source
    }

    // Returns a true FXProp only if either `lock`, `writer`, or `reader` is set.
    // Otherwise, returns `None`.
    pub(crate) fn syncish(&self) -> FXProp<bool> {
        *self.syncish.get_or_init(|| {
            self.source
                .lock()
                .as_ref()
                .and_then(|l| {
                    if *l.is_set() {
                        Some(l.into())
                    }
                    else {
                        None
                    }
                })
                .or_else(|| {
                    self.source.reader().as_ref().and_then(|r| {
                        if *r.is_set() {
                            Some(r.into())
                        }
                        else {
                            None
                        }
                    })
                })
                .or_else(|| {
                    self.source.writer().as_ref().and_then(|w| {
                        if *w.is_set() {
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

    pub(crate) fn helper_ident(&self, helper_kind: FXHelperKind) -> Option<&syn::Ident> {
        match helper_kind {
            FXHelperKind::Accessor => self.accessor_ident(),
            FXHelperKind::AccessorMut => self.accessor_mut_ident(),
            FXHelperKind::Builder => self.builder_ident(),
            FXHelperKind::Clearer => self.clearer_ident(),
            FXHelperKind::Lazy => self.lazy_ident(),
            FXHelperKind::Predicate => self.predicate_ident(),
            FXHelperKind::Reader => self.reader_ident(),
            FXHelperKind::Setter => self.setter_ident(),
            FXHelperKind::Writer => self.writer_ident(),
        }
    }

    pub(crate) fn skipped(&self) -> FXProp<bool> {
        *self.skipped.get_or_init(|| self.source.skip().into())
    }

    pub(crate) fn visibility(&self) -> Option<&syn::Visibility> {
        self.visibility
            .get_or_init(|| {
                if *self.source.private.is_true() {
                    return Some(syn::Visibility::Inherited);
                }
                self.source.visibility.as_ref().map(|v| v.value().clone())
            })
            .as_ref()
    }

    pub(crate) fn builder_attributes(&self) -> Option<&FXAttributes> {
        self.builder_attributes
            .get_or_init(|| self.source.builder().as_ref().and_then(|b| b.attributes()).cloned())
            .as_ref()
    }

    pub(crate) fn base_name(&self) -> Option<&syn::Ident> {
        self.base_name
            .get_or_init(|| {
                if let Some(ref bn) = self.source.base_name {
                    bn.value().map(|name| syn::Ident::new(name, bn.final_span()))
                }
                else {
                    None
                }
            })
            .as_ref()
    }

    pub(crate) fn default_value(&self) -> Option<&syn::Expr> {
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

    pub(crate) fn has_default(&self) -> FXProp<bool> {
        *self.has_default.get_or_init(|| {
            self.source
                .default_value()
                .as_ref()
                .map_or_else(|| false.into(), |d| d.is_set())
        })
    }

    pub(crate) fn doc(&self) -> &Vec<syn::Attribute> {
        self.doc.get_or_init(|| {
            self.source
                .attrs()
                .iter()
                .filter(|a| a.path().is_ident("doc"))
                .cloned()
                .collect()
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde(&self) -> Option<FXProp<Option<bool>>> {
        *self.serde.get_or_init(|| {
            self.source
                .serde
                .as_ref()
                .and_then(|s| Some(s.is_serde().respan(s.orig_span())))
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize(&self) -> Option<FXProp<bool>> {
        *self
            .serialize
            .get_or_init(|| self.source.serde().as_ref().and_then(|s| s.needs_serialize()))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize(&self) -> Option<FXProp<bool>> {
        *self
            .deserialize
            .get_or_init(|| self.source.serde.as_ref().and_then(|s| s.needs_deserialize()))
    }
}
