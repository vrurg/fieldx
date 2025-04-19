use std::rc::Rc;
use std::rc::Weak;

// #[cfg(feature = "diagnostics")]
// use crate::helper::FXOrig;
use crate::ctx::FXCodeGenCtx;
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
use fieldx_aux::FXPropBool;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSpaned;
use once_cell::unsync::OnceCell;
use quote::format_ident;
use syn::spanned::Spanned;
use syn::Token;

use super::FXSArgs;

#[derive(Debug)]
pub(crate) struct FXArgProps {
    source:      FXSArgs,
    codegen_ctx: Weak<FXCodeGenCtx>,

    // Accessor helper standard properties
    accessor:                       OnceCell<Option<FXProp<bool>>>,
    accessor_visibility:            OnceCell<Option<syn::Visibility>>,
    accessor_ident:                 OnceCell<Option<syn::Ident>>,
    // Accessor helper extended properties
    accessor_mode:                  OnceCell<Option<FXProp<FXAccessorMode>>>,
    // Mutable accessor helper standard properties
    accessor_mut:                   OnceCell<Option<FXProp<bool>>>,
    accessor_mut_visibility:        OnceCell<Option<syn::Visibility>>,
    accessor_mut_ident:             OnceCell<Option<syn::Ident>>,
    // Builder helper standard properties
    builder:                        OnceCell<Option<FXProp<bool>>>,
    builder_visibility:             OnceCell<Option<syn::Visibility>>,
    builder_ident:                  OnceCell<syn::Ident>,
    // Builder helper extended properties
    builder_doc:                    OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    builder_into:                   OnceCell<Option<FXProp<bool>>>,
    builder_method_doc:             OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    builder_opt_in:                 OnceCell<FXProp<bool>>,
    builder_prefix:                 OnceCell<Option<syn::Ident>>,
    builder_required:               OnceCell<Option<FXProp<bool>>>,
    // Builder struct properties are the ultimate factor in determining whether a builder struct is needed.
    builder_struct:                 OnceCell<FXProp<bool>>,
    builder_struct_attributes:      OnceCell<Option<FXAttributes>>,
    builder_struct_attributes_impl: OnceCell<Option<FXAttributes>>,
    builder_struct_visibility:      OnceCell<syn::Visibility>,
    // Clearer helper standard properties
    clearer:                        OnceCell<Option<FXProp<bool>>>,
    clearer_visibility:             OnceCell<Option<syn::Visibility>>,
    clearer_ident:                  OnceCell<Option<syn::Ident>>,
    // Predicate helper standard properties
    predicate:                      OnceCell<Option<FXProp<bool>>>,
    predicate_visibility:           OnceCell<Option<syn::Visibility>>,
    predicate_ident:                OnceCell<Option<syn::Ident>>,
    // Reader helper standard properties
    reader:                         OnceCell<Option<FXProp<bool>>>,
    reader_visibility:              OnceCell<Option<syn::Visibility>>,
    reader_ident:                   OnceCell<Option<syn::Ident>>,
    // Setter helper standard properties
    setter:                         OnceCell<Option<FXProp<bool>>>,
    setter_visibility:              OnceCell<Option<syn::Visibility>>,
    setter_ident:                   OnceCell<Option<syn::Ident>>,
    // Writer helper standard properties
    writer:                         OnceCell<Option<FXProp<bool>>>,
    writer_visibility:              OnceCell<Option<syn::Visibility>>,
    writer_ident:                   OnceCell<Option<syn::Ident>>,
    // Lazy helper standard properties
    lazy:                           OnceCell<Option<FXProp<bool>>>,
    lazy_visibility:                OnceCell<Option<syn::Visibility>>,
    lazy_ident:                     OnceCell<Option<syn::Ident>>,
    // Reference counted object helper standard properties
    rc:                             OnceCell<FXProp<bool>>,
    rc_visibility:                  OnceCell<Option<syn::Visibility>>,
    rc_doc:                         OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
    // Constructor new properties
    needs_new:                      OnceCell<FXProp<bool>>,
    new_visibility:                 OnceCell<Option<syn::Visibility>>,
    new_ident:                      OnceCell<Option<syn::Ident>>,
    // Other properties
    fallible:                       OnceCell<Option<FXProp<FXFallible>>>,
    inner_mut:                      OnceCell<Option<FXProp<bool>>>,
    into:                           OnceCell<Option<FXProp<bool>>>,
    lock:                           OnceCell<Option<FXProp<bool>>>,
    mode_async:                     OnceCell<Option<FXProp<bool>>>,
    mode_plain:                     OnceCell<Option<FXProp<bool>>>,
    mode_sync:                      OnceCell<Option<FXProp<bool>>>,
    needs_default:                  OnceCell<FXProp<bool>>,
    optional:                       OnceCell<Option<FXProp<bool>>>,
    setter_into:                    OnceCell<Option<FXProp<bool>>>,
    syncish:                        OnceCell<FXProp<bool>>,
    visibility:                     OnceCell<Option<syn::Visibility>>,
    has_post_build:                 OnceCell<FXProp<bool>>,
    post_build_ident:               OnceCell<Option<syn::Ident>>,
    myself_name:                    OnceCell<Option<syn::Ident>>,
    myself_downgrade_name:          OnceCell<Option<syn::Ident>>,
    myself_field_ident:             OnceCell<Option<syn::Ident>>,
    builder_has_error_type:         OnceCell<FXProp<bool>>,
    builder_error_type:             OnceCell<Option<syn::Path>>,
    builder_error_variant:          OnceCell<Option<syn::Path>>,

    #[cfg(feature = "serde")]
    serde:                    OnceCell<FXProp<bool>>,
    #[cfg(feature = "serde")]
    serialize:                OnceCell<Option<FXProp<bool>>>,
    #[cfg(feature = "serde")]
    deserialize:              OnceCell<Option<FXProp<bool>>>,
    #[cfg(feature = "serde")]
    serde_visibility:         OnceCell<Option<syn::Visibility>>,
    #[cfg(feature = "serde")]
    serde_default_value:      OnceCell<Option<FXDefault>>,
    #[cfg(feature = "serde")]
    serde_shadow_ident:       OnceCell<Option<syn::Ident>>,
    #[cfg(feature = "serde")]
    serde_rename_serialize:   OnceCell<Option<FXProp<String>>>,
    #[cfg(feature = "serde")]
    serde_rename_deserialize: OnceCell<Option<FXProp<String>>>,
    #[cfg(feature = "serde")]
    serde_doc:                OnceCell<Option<FXProp<Vec<syn::LitStr>>>>,
}

impl FXArgProps {
    simple_bool_prop! {builder}

    common_prop_impl! {
        accessor, accessor_mut, setter, clearer, predicate, reader, writer, lazy
    }

    doc_props! {
        builder_doc from builder.doc;
        builder_method_doc from builder.method_doc;
        rc_doc from rc.doc;
    }

    #[cfg(feature = "serde")]
    doc_props! { serde_doc from serde.doc; }

    simple_type_prop! {
        fallible, FXFallible;
    }

    pub(crate) fn new(args: FXSArgs, codegen_ctx: Weak<FXCodeGenCtx>) -> Self {
        Self {
            source: args,
            codegen_ctx,

            accessor: OnceCell::new(),
            accessor_visibility: OnceCell::new(),
            accessor_ident: OnceCell::new(),
            accessor_mode: OnceCell::new(),
            accessor_mut: OnceCell::new(),
            accessor_mut_visibility: OnceCell::new(),
            accessor_mut_ident: OnceCell::new(),
            builder: OnceCell::new(),
            builder_doc: OnceCell::new(),
            builder_ident: OnceCell::new(),
            builder_into: OnceCell::new(),
            builder_method_doc: OnceCell::new(),
            builder_opt_in: OnceCell::new(),
            builder_prefix: OnceCell::new(),
            builder_required: OnceCell::new(),
            builder_visibility: OnceCell::new(),
            builder_struct: OnceCell::new(),
            builder_struct_visibility: OnceCell::new(),
            builder_struct_attributes: OnceCell::new(),
            builder_struct_attributes_impl: OnceCell::new(),
            clearer: OnceCell::new(),
            clearer_visibility: OnceCell::new(),
            clearer_ident: OnceCell::new(),
            predicate: OnceCell::new(),
            predicate_visibility: OnceCell::new(),
            predicate_ident: OnceCell::new(),
            reader: OnceCell::new(),
            reader_visibility: OnceCell::new(),
            reader_ident: OnceCell::new(),
            setter: OnceCell::new(),
            setter_visibility: OnceCell::new(),
            setter_ident: OnceCell::new(),
            writer: OnceCell::new(),
            writer_visibility: OnceCell::new(),
            writer_ident: OnceCell::new(),
            lazy: OnceCell::new(),
            lazy_visibility: OnceCell::new(),
            lazy_ident: OnceCell::new(),
            fallible: OnceCell::new(),
            inner_mut: OnceCell::new(),
            into: OnceCell::new(),
            lock: OnceCell::new(),
            mode_async: OnceCell::new(),
            mode_plain: OnceCell::new(),
            mode_sync: OnceCell::new(),
            needs_new: OnceCell::new(),
            new_visibility: OnceCell::new(),
            new_ident: OnceCell::new(),
            needs_default: OnceCell::new(),
            optional: OnceCell::new(),
            setter_into: OnceCell::new(),
            syncish: OnceCell::new(),
            visibility: OnceCell::new(),
            has_post_build: OnceCell::new(),
            post_build_ident: OnceCell::new(),
            rc: OnceCell::new(),
            rc_visibility: OnceCell::new(),
            rc_doc: OnceCell::new(),
            myself_name: OnceCell::new(),
            myself_downgrade_name: OnceCell::new(),
            myself_field_ident: OnceCell::new(),
            builder_error_type: OnceCell::new(),
            builder_error_variant: OnceCell::new(),
            builder_has_error_type: OnceCell::new(),

            #[cfg(feature = "serde")]
            serde: OnceCell::new(),
            #[cfg(feature = "serde")]
            serialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            deserialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_visibility: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_default_value: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_shadow_ident: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_rename_serialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_rename_deserialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_doc: OnceCell::new(),
        }
    }

    pub(crate) fn codegen_ctx(&self) -> Rc<FXCodeGenCtx> {
        self.codegen_ctx.upgrade().expect("codegen context was dropped")
    }

    // Helper ident at the struct level defines helper method prefix.
    pub(crate) fn helper_ident(&self, helper_kind: FXHelperKind) -> Option<&syn::Ident> {
        match helper_kind {
            FXHelperKind::Accessor => self.accessor_ident(),
            FXHelperKind::AccessorMut => self.accessor_mut_ident(),
            FXHelperKind::Builder => self.builder_prefix(),
            FXHelperKind::Clearer => self.clearer_ident(),
            FXHelperKind::Lazy => self.lazy_ident(),
            FXHelperKind::Predicate => self.predicate_ident(),
            FXHelperKind::Reader => self.reader_ident(),
            FXHelperKind::Setter => self.setter_ident(),
            FXHelperKind::Writer => self.writer_ident(),
        }
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

    fn set_builder_opt_in(&self, value: FXProp<bool>) {
        // Ignore the error if the value was previously set.
        let _ = self.builder_opt_in.set(value);
    }

    pub(crate) fn needs_new(&self) -> FXProp<bool> {
        *self.needs_new.get_or_init(|| {
            self.source
                .new()
                .as_ref()
                .map_or_else(|| self.source.no_new().is_true().not(), |n| n.is_set())
        })
    }

    pub(crate) fn new_visibility(&self) -> Option<&syn::Visibility> {
        self.new_visibility
            .get_or_init(|| {
                let needs_new = self.needs_new();
                if *needs_new {
                    self.source
                        .new()
                        .as_ref()
                        .and_then(|n| n.visibility().cloned())
                        .or_else(|| Some(syn::Visibility::Public(Token![pub](needs_new.fx_span()))))
                }
                else {
                    None
                }
            })
            .as_ref()
    }

    // When the name of constructor method is not explicitly specified by the user then it is chosen based on the
    // method visibility. `__fieldx_new` is used for private methods and `new` for public ones.
    pub(crate) fn new_ident(&self) -> Option<&syn::Ident> {
        self.new_ident
            .get_or_init(|| {
                if *self.needs_new() {
                    let explicit_name = self.source.new().as_ref().and_then(|n| n.name());
                    Some(explicit_name.map_or_else(
                        || {
                            let span = self.needs_new().fx_span();
                            format_ident!(
                                "{}",
                                if self.new_visibility() == Some(&syn::Visibility::Inherited) {
                                    "__fieldx_new"
                                }
                                else {
                                    "new"
                                },
                                span = span
                            )
                        },
                        |name| format_ident!("{}", name.value(), span = name.final_span()),
                    ))
                }
                else {
                    None
                }
            })
            .as_ref()
    }

    pub(crate) fn builder_visibility(&self) -> Option<&syn::Visibility> {
        self.builder_visibility
            .get_or_init(|| self.visibility_of(&self.source.builder))
            .as_ref()
    }

    pub(crate) fn builder_attributes_fn(&self) -> Option<&FXAttributes> {
        self.source.builder.as_ref().and_then(|h| h.attributes_fn())
    }

    pub(crate) fn builder_opt_in(&self) -> FXProp<bool> {
        *self.builder_opt_in.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .map_or(FXProp::new(false, None), |b| b.opt_in().is_set().respan(b.orig_span()))
        })
    }

    pub(crate) fn builder_prefix(&self) -> Option<&syn::Ident> {
        self.builder_prefix
            .get_or_init(|| {
                self.source.builder().as_ref().and_then(|b| {
                    b.prefix()
                        .as_ref()
                        .and_then(|p| p.value().map(|v| syn::Ident::new(&v, p.final_span())))
                })
            })
            .as_ref()
    }

    pub(crate) fn builder_struct(&self) -> FXProp<bool> {
        *self.builder_struct.get_or_init(|| {
            self.builder().unwrap_or_else(|| -> FXProp<bool> {
                for fctx in self.codegen_ctx().all_field_ctx() {
                    let builder = fctx.builder();
                    if *builder {
                        self.set_builder_opt_in(builder);
                        return builder;
                    }
                }
                FXProp::new(false, None)
            })
        })
    }

    pub(crate) fn builder_struct_visibility(&self) -> &syn::Visibility {
        self.builder_struct_visibility.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .and_then(|b| b.visibility())
                .cloned()
                .unwrap_or_else(|| self.codegen_ctx().input().vis().clone())
        })
    }

    pub(crate) fn builder_struct_attributes(&self) -> Option<&FXAttributes> {
        self.builder_struct_attributes
            .get_or_init(|| {
                self.source
                    .builder()
                    .as_ref()
                    .and_then(|b| b.attributes())
                    .or_else(|| self.source.attributes().as_ref())
                    .cloned()
            })
            .as_ref()
    }

    pub(crate) fn builder_struct_attributes_impl(&self) -> Option<&FXAttributes> {
        self.builder_struct_attributes_impl
            .get_or_init(|| {
                self.source
                    .builder()
                    .as_ref()
                    .and_then(|b| b.attributes_impl())
                    .or_else(|| self.source.attributes_impl().as_ref())
                    .cloned()
            })
            .as_ref()
    }

    // Try to infer which mode applies to the struct. If it is explicitly declared as "sync" or "async", there is no
    // ambiguity.  Otherwise, check if any field explicitly requests sync mode.
    pub(crate) fn syncish(&self) -> FXProp<bool> {
        *self.syncish.get_or_init(|| {
            self.mode_sync().unwrap_or_else(|| {
                for fctx in self.codegen_ctx().all_field_ctx() {
                    let syncish = fctx
                        .mode_sync()
                        .or(fctx.mode_async())
                        .or(fctx.props().field_props().syncish());
                    if *syncish {
                        return syncish;
                    }
                }
                FXProp::new(false, None)
            })
        })
    }

    pub(crate) fn needs_default(&self) -> FXProp<bool> {
        *self.needs_default.get_or_init(|| {
            self.source.default().as_ref().map_or_else(
                || {
                    let is_syncish = *self.syncish();

                    #[cfg(feature = "serde")]
                    if self.serde_default_value().is_some() {
                        return FXProp::new(true, self.serde_default_value().orig_span());
                    }

                    if let Some(prop) = self.codegen_ctx().all_field_ctx().iter().find_map(|fctx| {
                        let has_default = fctx.props().field_props().has_default();
                        if *has_default {
                            return Some(has_default);
                        }
                        let lazy = fctx.lazy();
                        if is_syncish && *lazy {
                            return Some(lazy);
                        }
                        None
                    }) {
                        return prop;
                    }

                    false.into()
                },
                |d| d.is_set(),
            )
        })
    }

    pub(crate) fn has_post_build(&self) -> FXProp<bool> {
        *self.has_post_build.get_or_init(|| {
            self.source.builder().as_ref().map_or_else(
                || FXProp::new(false, None),
                |b| b.has_post_build().respan(b.orig_span()),
            )
        })
    }

    pub(crate) fn post_build_ident(&self) -> Option<&syn::Ident> {
        self.post_build_ident
            .get_or_init(|| {
                if *self.has_post_build() {
                    Some(
                        self.source
                            .builder()
                            .as_ref()
                            .and_then(|b| b.post_build())
                            .and_then(|pb| pb.value().cloned())
                            .unwrap_or_else(|| {
                                let span = self.has_post_build().fx_span();
                                format_ident!("post_build", span = span)
                            }),
                    )
                }
                else {
                    None
                }
            })
            .as_ref()
    }

    pub(crate) fn rc(&self) -> FXProp<bool> {
        *self
            .rc
            .get_or_init(|| self.source.rc().as_ref().map_or_else(|| false.into(), |rc| rc.is_set()))
    }

    pub(crate) fn rc_visibility(&self) -> Option<&syn::Visibility> {
        self.rc_visibility
            .get_or_init(|| self.source.rc().as_ref().and_then(|rc| rc.visibility()).cloned())
            .as_ref()
    }

    pub(crate) fn myself_name(&self) -> Option<&syn::Ident> {
        self.myself_name
            .get_or_init(|| {
                self.source.rc().as_ref().map(|rc| {
                    rc.name().map_or_else(
                        || format_ident!("myself", span = rc.final_span()),
                        |name| format_ident!("{}", name.value(), span = name.final_span()),
                    )
                })
            })
            .as_ref()
    }

    pub(crate) fn myself_downgrade_name(&self) -> Option<&syn::Ident> {
        self.myself_downgrade_name
            .get_or_init(|| {
                self.myself_name()
                    .map(|name| format_ident!("{}_downgrade", name, span = name.span()))
            })
            .as_ref()
    }

    pub(crate) fn myself_field_ident(&self) -> Option<&syn::Ident> {
        self.myself_field_ident
            .get_or_init(|| {
                self.myself_name()
                    .map(|name| format_ident!("__weak_{}", name, span = name.span()))
            })
            .as_ref()
    }

    pub(crate) fn builder_error_type(&self) -> Option<&syn::Path> {
        self.builder_error_type
            .get_or_init(|| self.source.builder().as_ref().and_then(|b| b.error_type().cloned()))
            .as_ref()
    }

    pub(crate) fn builder_has_error_type(&self) -> FXProp<bool> {
        *self.builder_has_error_type.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .and_then(|b| b.error_type().map(|et| FXProp::new(true, Some(et.span()))))
                .unwrap_or_else(|| FXProp::new(false, None))
        })
    }

    pub(crate) fn builder_error_variant(&self) -> Option<&syn::Path> {
        self.builder_error_variant
            .get_or_init(|| self.source.builder().as_ref().and_then(|b| b.error_variant().cloned()))
            .as_ref()
    }

    pub(crate) fn builder_ident(&self) -> &syn::Ident {
        self.builder_ident.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .and_then(|b| {
                    b.name()
                        .map(|name| format_ident!("{}", name.value(), span = name.final_span()))
                })
                .unwrap_or_else(|| {
                    let codegen_ctx = self.codegen_ctx();
                    let input_ident = codegen_ctx.input().ident();
                    format_ident!("{}Builder", input_ident, span = input_ident.span())
                })
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde(&self) -> FXProp<bool> {
        *self.serde.get_or_init(|| {
            self.source.serde.as_ref().map_or_else(
                || FXProp::new(false, None),
                |s| {
                    let is_serde = s.is_serde();
                    FXProp::new(
                        is_serde.value().unwrap_or(true),
                        is_serde.orig_span().or_else(|| s.orig_span()),
                    )
                },
            )
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize(&self) -> Option<FXProp<bool>> {
        *self.serialize.get_or_init(|| {
            self.source
                .serde()
                .as_ref()
                .and_then(|s| s.needs_serialize())
                .or_else(|| {
                    for fctx in self.codegen_ctx().all_field_ctx() {
                        if let Some(serialize) = fctx.props().field_props().serialize() {
                            if *serialize {
                                return Some(serialize);
                            }
                        }
                    }
                    None
                })
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize(&self) -> Option<FXProp<bool>> {
        *self.deserialize.get_or_init(|| {
            self.source
                .serde()
                .as_ref()
                .and_then(|s| s.needs_deserialize())
                .or_else(|| {
                    for fctx in self.codegen_ctx().all_field_ctx() {
                        if let Some(deserialize) = fctx.props().field_props().deserialize() {
                            if *deserialize {
                                return Some(deserialize);
                            }
                        }
                    }
                    None
                })
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn needs_serialize(&self) -> FXProp<bool> {
        self.serialize().unwrap_or_else(|| self.serde())
    }

    #[cfg(feature = "serde")]
    pub(crate) fn needs_deserialize(&self) -> FXProp<bool> {
        self.deserialize().unwrap_or_else(|| self.serde())
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_shadow_ident(&self) -> Option<&syn::Ident> {
        self.serde_shadow_ident
            .get_or_init(|| {
                if *self.serde() {
                    self.source
                        .serde()
                        .as_ref()
                        .and_then(|s| {
                            s.shadow_name()
                                .as_ref()
                                .and_then(|sn| sn.value().map(|name| format_ident!("{}", name, span = sn.final_span())))
                        })
                        .or_else(|| {
                            let codegen_ctx = self.codegen_ctx();
                            let input_ident = codegen_ctx.input().ident();
                            Some(format_ident!("__{}Shadow", input_ident, span = input_ident.span()))
                        })
                }
                else {
                    None
                }
            })
            .as_ref()
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_visibility(&self) -> Option<&syn::Visibility> {
        self.serde_visibility
            .get_or_init(|| self.source.serde().as_ref().and_then(|s| s.visibility()).cloned())
            .as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn base_name(&self) -> Option<syn::Ident> {
        Some(self.codegen_ctx().input().ident().clone())
    }
}
