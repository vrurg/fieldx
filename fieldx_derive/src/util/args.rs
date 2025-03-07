use std::rc::Rc;

// #[cfg(feature = "diagnostics")]
// use crate::helper::FXOrig;
use crate::{
    ctx::FXCodeGenCtx,
    helper::{FXHelperContainer, FXHelperKind},
    util::{common_prop_impl, mode_async_prop, mode_plain_prop, mode_sync_prop, simple_bool_prop, simple_type_prop},
};
use darling::FromMeta;
use fieldx_aux::{
    validate_exclusives, FXAccessor, FXAccessorMode, FXAttributes, FXBool, FXBoolHelper, FXBuilder, FXFallible,
    FXHelper, FXHelperTrait, FXNestingAttr, FXOrig, FXProp, FXPropBool, FXSerde, FXSetter, FXSynValue, FXSyncMode,
    FXTriggerHelper,
};
use getset::Getters;
use once_cell::unsync::OnceCell;
use proc_macro2::Span;
use quote::format_ident;
use syn::spanned::Spanned;

use super::helper_standard_methods;

#[derive(Debug, FromMeta, Clone, Getters, Default)]
#[darling(and_then = Self::validate)]
#[getset(get = "pub")]
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
    visibility:   Option<FXSynValue<syn::Visibility>>,
    #[getset(get = "pub with_prefix")]
    clone:        Option<FXBool>,
    #[getset(get = "pub with_prefix")]
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
    );

    // Generate needs_<helper> methods
    // needs_helper! {accessor, accessor_mut, setter, reader, writer, clearer, predicate}

    #[inline]
    pub fn validate(self) -> Result<Self, darling::Error> {
        let mut acc = darling::Error::accumulator();
        if let Err(err) = self
            .validate_exclusives()
            .map_err(|err| err.with_span(&Span::call_site()))
        {
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
    // pub fn is_ref_counted(&self) -> bool {
    //     self.rc.is_true()
    // }

    // #[inline]
    // pub fn is_builder_into(&self) -> Option<bool> {
    //     self.builder.as_ref().and_then(|h| h.is_into())
    // }

    // #[inline]
    // pub fn is_builder_required(&self) -> Option<bool> {
    //     self.builder.as_ref().and_then(|h| h.is_required())
    // }

    // #[inline]
    // pub fn needs_new(&self) -> bool {
    //     // new() is not possible without Default implementation
    //     self.no_new.not_true() && self.needs_default().unwrap_or(true)
    // }

    // #[inline]
    // pub fn needs_default(&self) -> Option<bool> {
    //     self.default.is_true_opt()
    // }

    // #[inline]
    // pub fn accessor_mode(&self) -> Option<FXAccessorMode> {
    //     self.accessor.as_ref().and_then(|h| h.mode())
    // }

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
    pub fn fallible_error(&self) -> Option<&syn::Path> {
        self.fallible.as_ref().and_then(|f| f.error_type().map(|et| et.value()))
    }

    #[inline]
    pub fn rc_span(&self) -> Span {
        self.rc
            .as_ref()
            .and_then(|r| r.orig_span())
            .unwrap_or_else(|| Span::call_site())
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn serde_helper_span(&self) -> Span {
        self.serde
            .as_ref()
            .and_then(|sw| sw.orig_span())
            .unwrap_or_else(|| Span::call_site())
    }
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

#[derive(Debug)]
pub struct FXArgProps {
    source:      FXSArgs,
    codegen_ctx: Rc<FXCodeGenCtx>,

    // Accessor helper standard properties
    accessor:                  OnceCell<Option<FXProp<bool>>>,
    accessor_visibility:       OnceCell<Option<syn::Visibility>>,
    accessor_ident:            OnceCell<Option<syn::Ident>>,
    // Accessor helper extended properties
    accessor_mode:             OnceCell<Option<FXProp<FXAccessorMode>>>,
    // Mutable accessor helper standard properties
    accessor_mut:              OnceCell<Option<FXProp<bool>>>,
    accessor_mut_visibility:   OnceCell<Option<syn::Visibility>>,
    accessor_mut_ident:        OnceCell<Option<syn::Ident>>,
    // Builder helper standard properties
    builder:                   OnceCell<Option<FXProp<bool>>>,
    builder_visibility:        OnceCell<Option<syn::Visibility>>,
    builder_ident:             OnceCell<Option<syn::Ident>>,
    // Builder helper extended properties
    builder_into:              OnceCell<Option<FXProp<bool>>>,
    builder_required:          OnceCell<Option<FXProp<bool>>>,
    builder_opt_in:            OnceCell<FXProp<bool>>,
    builder_struct:            OnceCell<FXProp<bool>>,
    builder_struct_ident:      OnceCell<syn::Ident>,
    builder_struct_visibility: OnceCell<syn::Visibility>,
    // Clearer helper standard properties
    clearer:                   OnceCell<Option<FXProp<bool>>>,
    clearer_visibility:        OnceCell<Option<syn::Visibility>>,
    clearer_ident:             OnceCell<Option<syn::Ident>>,
    // Predicate helper standard properties
    predicate:                 OnceCell<Option<FXProp<bool>>>,
    predicate_visibility:      OnceCell<Option<syn::Visibility>>,
    predicate_ident:           OnceCell<Option<syn::Ident>>,
    // Reader helper standard properties
    reader:                    OnceCell<Option<FXProp<bool>>>,
    reader_visibility:         OnceCell<Option<syn::Visibility>>,
    reader_ident:              OnceCell<Option<syn::Ident>>,
    // Setter helper standard properties
    setter:                    OnceCell<Option<FXProp<bool>>>,
    setter_visibility:         OnceCell<Option<syn::Visibility>>,
    setter_ident:              OnceCell<Option<syn::Ident>>,
    // Writer helper standard properties
    writer:                    OnceCell<Option<FXProp<bool>>>,
    writer_visibility:         OnceCell<Option<syn::Visibility>>,
    writer_ident:              OnceCell<Option<syn::Ident>>,
    // Lazy helper standard properties
    lazy:                      OnceCell<Option<FXProp<bool>>>,
    lazy_visibility:           OnceCell<Option<syn::Visibility>>,
    lazy_ident:                OnceCell<Option<syn::Ident>>,
    // Reference counted object helper standard properties
    rc:                        OnceCell<FXProp<bool>>,
    rc_visibility:             OnceCell<Option<syn::Visibility>>,
    rc_ident:                  OnceCell<Option<syn::Ident>>,
    // Other properties
    fallible:                  OnceCell<Option<FXProp<FXFallible>>>,
    inner_mut:                 OnceCell<Option<FXProp<bool>>>,
    into:                      OnceCell<Option<FXProp<bool>>>,
    lock:                      OnceCell<Option<FXProp<bool>>>,
    mode_async:                OnceCell<Option<FXProp<bool>>>,
    mode_plain:                OnceCell<Option<FXProp<bool>>>,
    mode_sync:                 OnceCell<Option<FXProp<bool>>>,
    needs_new:                 OnceCell<FXProp<bool>>,
    needs_default:             OnceCell<FXProp<bool>>,
    optional:                  OnceCell<Option<FXProp<bool>>>,
    setter_into:               OnceCell<Option<FXProp<bool>>>,
    syncish:                   OnceCell<FXProp<bool>>,
    visibility:                OnceCell<Option<syn::Visibility>>,
    has_post_build:            OnceCell<FXProp<bool>>,
    post_build_ident:          OnceCell<Option<syn::Ident>>,
    myself_name:               OnceCell<Option<syn::Ident>>,
    myself_downgrade_name:     OnceCell<Option<syn::Ident>>,
    myself_field_ident:        OnceCell<Option<syn::Ident>>,
    builder_has_error_type:    OnceCell<FXProp<bool>>,
    builder_error_type:        OnceCell<Option<syn::Path>>,
    builder_error_variant:     OnceCell<Option<syn::Path>>,

    #[cfg(feature = "serde")]
    serde:       OnceCell<FXProp<bool>>,
    #[cfg(feature = "serde")]
    serialize:   OnceCell<Option<FXProp<bool>>>,
    #[cfg(feature = "serde")]
    deserialize: OnceCell<Option<FXProp<bool>>>,
}

impl FXArgProps {
    common_prop_impl! {}

    simple_type_prop! {
        fallible, FXFallible;
    }

    pub fn new(args: FXSArgs, codegen_ctx: Rc<FXCodeGenCtx>) -> Self {
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
            builder_visibility: OnceCell::new(),
            builder_ident: OnceCell::new(),
            builder_into: OnceCell::new(),
            builder_required: OnceCell::new(),
            builder_opt_in: OnceCell::new(),
            builder_struct: OnceCell::new(),
            builder_struct_ident: OnceCell::new(),
            builder_struct_visibility: OnceCell::new(),
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
            needs_default: OnceCell::new(),
            optional: OnceCell::new(),
            setter_into: OnceCell::new(),
            syncish: OnceCell::new(),
            visibility: OnceCell::new(),
            has_post_build: OnceCell::new(),
            post_build_ident: OnceCell::new(),
            rc: OnceCell::new(),
            rc_visibility: OnceCell::new(),
            rc_ident: OnceCell::new(),
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
        }
    }

    pub fn builder_opt_in(&self) -> FXProp<bool> {
        *self.builder_opt_in.get_or_init(|| {
            self.source.builder().as_ref().map_or_else(
                || FXProp::new(false, None),
                |b| b.is_builder_opt_in().respan(b.orig_span()),
            )
        })
    }

    pub fn visibility(&self) -> Option<&syn::Visibility> {
        self.visibility
            .get_or_init(|| self.source.visibility.as_ref().map(|v| v.value().clone()))
            .as_ref()
    }

    fn set_builder_opt_in(&self, value: FXProp<bool>) {
        self.builder_opt_in.set(value);
    }

    pub fn needs_new(&self) -> FXProp<bool> {
        *self.needs_new.get_or_init(|| self.source.no_new().is_true().not())
    }

    pub fn builder_struct(&self) -> FXProp<bool> {
        *self.builder_struct.get_or_init(|| {
            self.builder().unwrap_or_else(|| -> FXProp<bool> {
                for fctx in self.codegen_ctx.all_field_ctx() {
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

    pub fn builder_struct_visibility(&self) -> &syn::Visibility {
        self.builder_struct_visibility.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .and_then(|b| b.visibility())
                .cloned()
                .unwrap_or_else(|| self.codegen_ctx.input().vis().clone())
        })
    }

    // Try to infer which mode applies to the struct. If it is explicitly declared as "sync" or "async", there is no
    // ambiguity.  Otherwise, check if any field explicitly requests sync mode.
    pub fn syncish(&self) -> FXProp<bool> {
        *self.syncish.get_or_init(|| {
            self.mode_sync().unwrap_or_else(|| {
                for fctx in self.codegen_ctx.all_field_ctx() {
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

    pub fn needs_default(&self) -> FXProp<bool> {
        *self.needs_default.get_or_init(|| {
            self.source.default().as_ref().map_or_else(
                || {
                    let needs_new = self.needs_new();
                    if *needs_new {
                        return needs_new;
                    }

                    let is_syncish = *self.syncish();

                    if let Some(prop) = self.codegen_ctx.all_field_ctx().iter().find_map(|fctx| {
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
                |d| d.is_true(),
            )
        })
    }

    pub fn has_post_build(&self) -> FXProp<bool> {
        *self.has_post_build.get_or_init(|| {
            self.source.builder().as_ref().map_or_else(
                || FXProp::new(false, None),
                |b| b.has_post_build().respan(b.orig_span()),
            )
        })
    }

    pub fn post_build_ident(&self) -> Option<&syn::Ident> {
        self.post_build_ident
            .get_or_init(|| {
                if *self.has_post_build() {
                    Some(
                        self.source
                            .builder()
                            .as_ref()
                            .and_then(|b| b.post_build().as_ref())
                            .and_then(|pb| pb.value().cloned())
                            .unwrap_or_else(|| {
                                let span = self.has_post_build().span();
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

    pub fn rc(&self) -> FXProp<bool> {
        *self.rc.get_or_init(|| {
            self.source
                .rc()
                .as_ref()
                .map_or_else(|| false.into(), |rc| rc.is_true())
        })
    }

    pub fn rc_visibility(&self) -> Option<&syn::Visibility> {
        self.rc_visibility
            .get_or_init(|| self.source.rc().as_ref().and_then(|rc| rc.visibility()).cloned())
            .as_ref()
    }

    pub fn rc_ident(&self) -> Option<&syn::Ident> {
        self.rc_ident
            .get_or_init(|| {
                self.source.rc().as_ref().and_then(|rc| {
                    rc.name()
                        .map(|name| format_ident!("{}", name.value(), span = name.final_span()))
                })
            })
            .as_ref()
    }

    pub fn myself_name(&self) -> Option<&syn::Ident> {
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

    pub fn myself_downgrade_name(&self) -> Option<&syn::Ident> {
        self.myself_downgrade_name
            .get_or_init(|| {
                self.myself_name()
                    .map(|name| format_ident!("{}_downgrade", name, span = name.span()))
            })
            .as_ref()
    }

    pub fn myself_field_ident(&self) -> Option<&syn::Ident> {
        self.myself_field_ident
            .get_or_init(|| {
                self.myself_name()
                    .map(|name| format_ident!("__weak_{}", name, span = name.span()))
            })
            .as_ref()
    }

    pub fn builder_error_type(&self) -> Option<&syn::Path> {
        self.builder_error_type
            .get_or_init(|| self.source.builder().as_ref().and_then(|b| b.error_type().cloned()))
            .as_ref()
    }

    pub fn builder_has_error_type(&self) -> FXProp<bool> {
        *self.builder_has_error_type.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .and_then(|b| b.error_type().map(|et| FXProp::new(true, Some(et.span()))))
                .unwrap_or_else(|| FXProp::new(false, None))
        })
    }

    pub fn builder_error_variant(&self) -> Option<&syn::Path> {
        self.builder_error_variant
            .get_or_init(|| self.source.builder().as_ref().and_then(|b| b.error_variant().cloned()))
            .as_ref()
    }

    pub fn builder_struct_ident(&self) -> &syn::Ident {
        self.builder_struct_ident.get_or_init(|| {
            self.source
                .builder()
                .as_ref()
                .and_then(|b| {
                    b.name()
                        .map(|name| format_ident!("{}", name.value(), span = name.final_span()))
                })
                .unwrap_or_else(|| {
                    let input_ident = self.codegen_ctx.input().ident();
                    format_ident!("{}Builder", input_ident, span = input_ident.span())
                })
        })
    }

    #[cfg(feature = "serde")]
    pub fn serde(&self) -> FXProp<bool> {
        *self.serde.get_or_init(|| {
            self.source.serde().as_ref().map_or_else(
                || FXProp::new(false, None),
                |s| {
                    s.is_serde(s.orig_span())
                        // None happens when no explicit serialize or deserialize used. In this case, we consider it as
                        // true at the struct level.
                        .unwrap_or_else(|| FXProp::new(true, s.orig_span()))
                },
            )
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
            .get_or_init(|| self.source.serde().as_ref().and_then(|s| s.needs_deserialize()))
    }
}
