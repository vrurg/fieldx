use fieldx_aux::FXAccessorMode;
use fieldx_aux::FXAttributes;
#[cfg(feature = "serde")]
use fieldx_aux::FXDefault;
use fieldx_aux::FXHelperTrait;
use fieldx_aux::FXProp;
use fieldx_aux::FXPropBool;
use fieldx_aux::FXTriggerHelper;
use fieldx_derive_support::fallback_prop;
use once_cell::sync::OnceCell;
use quote::format_ident;
use std::rc::Rc;

use crate::ctx::FXCodeGenCtx;
use crate::field_receiver::FXFieldProps;
use crate::helper::FXHelperKind;
use crate::util::args::FXArgProps;

macro_rules! helper_visibility_method {
    ( $( $helper:ident ),* $(,)? ) => {
        $(
            ::paste::paste!{
                pub(crate) fn [<$helper _visibility>](&self) -> &syn::Visibility {
                    self.[<$helper _visibility>].get_or_init(|| {
                        self.helper_visibility(FXHelperKind::[<$helper:camel>])
                    })
                }
            }
        )*
    };
}

macro_rules! helper_ident_method {
    ( $( $helper:ident ),* $(,)? ) => {
        $(
            ::paste::paste!{
                pub(crate) fn [<$helper _ident>](&self) -> &syn::Ident {
                    self.[<$helper _ident>].get_or_init(|| {
                        self.helper_ident(FXHelperKind::[<$helper:camel>])
                    })
                }
            }
        )*
    };
}

#[derive(Debug)]
pub(crate) struct FieldCTXProps {
    field_props: FXFieldProps,
    arg_props:   Rc<FXArgProps>,
    codegen_ctx: Rc<FXCodeGenCtx>,

    // --- Final helper properties
    // Accessor helper standard properties
    accessor:                  OnceCell<FXProp<bool>>,
    accessor_visibility:       OnceCell<syn::Visibility>,
    accessor_ident:            OnceCell<syn::Ident>,
    // Accessor helper specific properties
    accessor_mode:             OnceCell<FXProp<FXAccessorMode>>,
    // Mutable accessor helper standard properties
    accessor_mut:              OnceCell<FXProp<bool>>,
    accessor_mut_visibility:   OnceCell<syn::Visibility>,
    accessor_mut_ident:        OnceCell<syn::Ident>,
    // Builder helper standard properties
    builder:                   OnceCell<FXProp<bool>>,
    /// Visibility of the builder method for this field on the builder object.
    builder_method_visibility: OnceCell<syn::Visibility>,
    builder_ident:             OnceCell<syn::Ident>,
    // Builder helper specific properties
    builder_into:              OnceCell<FXProp<bool>>,
    builder_required:          OnceCell<FXProp<bool>>,
    // If the field can obtain its value from sources other than the builder, or if it is optional, then calling its
    // builder method is optional.
    builder_method_optional:   OnceCell<FXProp<bool>>,
    // Clearer helper standard properties
    clearer:                   OnceCell<FXProp<bool>>,
    clearer_visibility:        OnceCell<syn::Visibility>,
    clearer_ident:             OnceCell<syn::Ident>,
    // Predicate helper standard properties
    predicate:                 OnceCell<FXProp<bool>>,
    predicate_visibility:      OnceCell<syn::Visibility>,
    predicate_ident:           OnceCell<syn::Ident>,
    // Reader helper standard properties
    reader:                    OnceCell<FXProp<bool>>,
    reader_visibility:         OnceCell<syn::Visibility>,
    reader_ident:              OnceCell<syn::Ident>,
    // Setter helper standard properties
    setter:                    OnceCell<FXProp<bool>>,
    setter_visibility:         OnceCell<syn::Visibility>,
    setter_ident:              OnceCell<syn::Ident>,
    // Setter helper specific properties
    setter_into:               OnceCell<FXProp<bool>>,
    // Writer helper standard properties
    writer:                    OnceCell<FXProp<bool>>,
    writer_visibility:         OnceCell<syn::Visibility>,
    writer_ident:              OnceCell<syn::Ident>,
    // Lazy helper standard properties
    lazy:                      OnceCell<FXProp<bool>>,
    lazy_ident:                OnceCell<syn::Ident>,
    // --- Other properties
    // The final base name of the field. Will be used in method name generation.
    base_name:                 OnceCell<syn::Ident>,
    fallible:                  OnceCell<FXProp<bool>>,
    fallible_error:            OnceCell<Option<syn::Path>>,
    forced_builder:            OnceCell<FXProp<bool>>,
    inner_mut:                 OnceCell<FXProp<bool>>,
    lock:                      OnceCell<FXProp<bool>>,
    mode_async:                OnceCell<FXProp<bool>>,
    mode_plain:                OnceCell<FXProp<bool>>,
    mode_sync:                 OnceCell<FXProp<bool>>,
    optional:                  OnceCell<FXProp<bool>>,

    #[cfg(feature = "serde")]
    serde:                    OnceCell<FXProp<bool>>,
    #[cfg(feature = "serde")]
    serialize:                OnceCell<FXProp<bool>>,
    #[cfg(feature = "serde")]
    deserialize:              OnceCell<FXProp<bool>>,
    #[cfg(feature = "serde")]
    /// Field is an Option in the shadow struct if it is optional or lazy and has no default value
    serde_optional:           OnceCell<FXProp<bool>>,
    #[cfg(feature = "serde")]
    serde_rename_serialize:   OnceCell<Option<FXProp<String>>>,
    #[cfg(feature = "serde")]
    serde_rename_deserialize: OnceCell<Option<FXProp<String>>>,
}

impl FieldCTXProps {
    fallback_prop! {
        accessor, bool, default {
            self.clearer().or(self.predicate()).or(self.lazy()).or(self.inner_mut())
        };

        accessor_mode,
            &FXAccessorMode,
            cloned, // Means that the value is cloned from the field or argument.
            default FXProp::new(FXAccessorMode::None, None);

        // Field is implicitly optional if either clearer or predicate are set, unless there is `lazy` which is then
        // taking over.
        optional, bool, default {
            if *self.lazy() {
                false.into()
            }
            else {
                let maybe = self.clearer().or(self.predicate());
                if *maybe {
                    maybe
                }
                else {
                    false.into()
                }
            }
        };

        lock, bool, default {
            self.reader().or(self.writer()).or(
                if *self.mode_sync() {
                    self.inner_mut()
                }
                else {
                    false.into()
                }
            )
        };

        accessor_mut, false;
        builder_into, false;
        builder_required, false;
        clearer, false;
        inner_mut, false;
        lazy, false;
        predicate, false;
        reader, false;
        setter, false;
        setter_into, false;
        writer, false;
    }

    helper_ident_method! { accessor, accessor_mut, clearer, lazy, predicate, reader, setter, writer }

    helper_visibility_method! { accessor, accessor_mut, clearer, predicate, reader, setter, writer }

    pub(crate) fn new(field: FXFieldProps, codegen_ctx: Rc<FXCodeGenCtx>) -> Self {
        Self {
            field_props: field,
            arg_props: codegen_ctx.arg_props().clone(),
            codegen_ctx,

            accessor: OnceCell::new(),
            accessor_visibility: OnceCell::new(),
            accessor_ident: OnceCell::new(),
            accessor_mode: OnceCell::new(),
            accessor_mut: OnceCell::new(),
            accessor_mut_visibility: OnceCell::new(),
            accessor_mut_ident: OnceCell::new(),
            builder: OnceCell::new(),
            builder_method_visibility: OnceCell::new(),
            builder_ident: OnceCell::new(),
            builder_into: OnceCell::new(),
            builder_required: OnceCell::new(),
            builder_method_optional: OnceCell::new(),
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
            setter_into: OnceCell::new(),
            writer: OnceCell::new(),
            writer_visibility: OnceCell::new(),
            writer_ident: OnceCell::new(),
            lazy: OnceCell::new(),
            lazy_ident: OnceCell::new(),
            base_name: OnceCell::new(),
            fallible: OnceCell::new(),
            fallible_error: OnceCell::new(),
            forced_builder: OnceCell::new(),
            inner_mut: OnceCell::new(),
            lock: OnceCell::new(),
            mode_async: OnceCell::new(),
            mode_plain: OnceCell::new(),
            mode_sync: OnceCell::new(),
            optional: OnceCell::new(),

            #[cfg(feature = "serde")]
            serde: OnceCell::new(),
            #[cfg(feature = "serde")]
            serialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            deserialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_optional: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_rename_serialize: OnceCell::new(),
            #[cfg(feature = "serde")]
            serde_rename_deserialize: OnceCell::new(),
        }
    }

    pub(crate) fn field_props(&self) -> &FXFieldProps {
        &self.field_props
    }

    pub(crate) fn arg_props(&self) -> &Rc<FXArgProps> {
        &self.arg_props
    }

    // Produce helper visibility by using the following order:
    // 1. field props:
    //    a. helper visibility
    //    b. default visibility
    // 2. args props:
    //    a. helper visibility
    //    b. default visibility
    // 3. field visibility unless "inherited/private"
    // 4. input struct visibility
    //
    // Fall back to the struct's visibility because helper methods (and builder) typically become part of the public API,
    // and therefore should adopt the visibility of the struct.
    //
    // Used by the helper_visibility! macro.
    fn helper_visibility(&self, helper_kind: FXHelperKind) -> syn::Visibility {
        self.field_props()
            .helper_visibility(helper_kind)
            .or_else(|| self.arg_props().helper_visibility(helper_kind))
            .cloned()
            .unwrap_or_else(|| {
                let mut vis = self.field_props.field().vis();
                if matches!(vis, syn::Visibility::Inherited) {
                    vis = self.codegen_ctx.input().vis();
                }
                vis.clone()
            })
    }

    fn helper_ident(&self, helper_kind: FXHelperKind) -> syn::Ident {
        self.field_props()
            .helper_ident(helper_kind)
            .cloned()
            .unwrap_or_else(|| {
                let base_name = self.base_name();
                let prefix = self
                    .arg_props()
                    .helper_ident(helper_kind)
                    .map_or_else(|| helper_kind.default_prefix().to_string(), |i| i.to_string());
                let suffix = helper_kind.default_suffix();
                format_ident!("{}{}{}", prefix, base_name, suffix, span = base_name.span())
            })
    }

    pub(crate) fn helper_attributes_fn(&self, helper_kind: FXHelperKind) -> Option<&FXAttributes> {
        self.field_props
            .helper_attributes_fn(helper_kind)
            .or_else(|| self.field_props.field().attributes_fn().as_ref())
            .or_else(|| self.arg_props.helper_attributes_fn(helper_kind))
            .or_else(|| self.codegen_ctx.args().attributes_fn().as_ref())
    }

    pub(crate) fn builder(&self) -> FXProp<bool> {
        *self.builder.get_or_init(|| {
            self.field_props()
                .builder()
                .or_else(|| {
                    let arg_props = self.arg_props();
                    arg_props.builder().and_then(|b| {
                        if *b && !*arg_props.builder_opt_in() {
                            Some(b)
                        }
                        else {
                            Some(b.not())
                        }
                    })
                })
                .unwrap_or_else(|| FXProp::new(false, *self.field_props.field().fieldx_attr_span()))
        })
    }

    pub(crate) fn builder_ident(&self) -> &syn::Ident {
        self.builder_ident.get_or_init(|| {
            self.field_props()
                .builder_ident()
                .unwrap_or_else(|| self.base_name())
                .clone()
        })
    }

    pub(crate) fn builder_method_visibility(&self) -> &syn::Visibility {
        self.builder_method_visibility
            .get_or_init(|| self.helper_visibility(FXHelperKind::Builder))
    }

    pub(crate) fn builder_method_optional(&self) -> FXProp<bool> {
        *self.builder_method_optional.get_or_init(|| {
            let mut vreq = self.builder_required();

            if !*vreq {
                vreq = self.builder();
                if *vreq {
                    // Let's see if there is a source for the field value or it is optional.
                    let vopt = self.optional().or(self.lazy());
                    if *vopt {
                        return vopt;
                    }
                    if let Some(default) = self.field_props().field().default_value() {
                        // Use `is_true` here because for a default value, `is_set` indicates that it has an explicit value.
                        // However, a plain `default` with no arguments simply means "we use ..Default::default()",
                        // which also counts as an extra field value source.
                        let vopt = default.is_true();
                        if *vopt {
                            return vopt;
                        }
                    }
                }
            }

            vreq.not().respan(Some(*self.field_props().field().span()))
        })
    }

    // A special case when the builder is forced by the field attribute.
    pub(crate) fn forced_builder(&self) -> FXProp<bool> {
        *self.forced_builder.get_or_init(|| {
            self.field_props
                .field()
                .builder()
                .as_ref()
                .and_then(|b| b.name().map(|n| FXProp::new(true, n.orig_span())))
                .unwrap_or_else(|| FXProp::new(false, *self.field_props.field().fieldx_attr_span()))
        })
    }

    // Fallible is specific because it can be enabled on the field level, but the error type can be defined on the
    // struct level.
    pub(crate) fn fallible(&self) -> FXProp<bool> {
        *self.fallible.get_or_init(|| {
            self.field_props()
                .fallible()
                .map(|f| f.into())
                .or_else(|| self.arg_props().fallible().map(|f| f.into()))
                .unwrap_or_else(|| FXProp::new(false, *self.field_props.field().fieldx_attr_span()))
        })
    }

    pub(crate) fn fallible_error(&self) -> Option<&syn::Path> {
        self.fallible_error
            .get_or_init(|| {
                self.field_props()
                    .fallible()
                    .and_then(|f| f.value().error_type().map(|et| et.value().clone()))
                    .or_else(|| {
                        self.arg_props()
                            .fallible()
                            .and_then(|f| f.value().error_type().map(|et| et.value().clone()))
                    })
            })
            .as_ref()
    }

    pub(crate) fn mode_sync(&self) -> FXProp<bool> {
        *self.mode_sync.get_or_init(|| {
            let field_props = self.field_props();
            field_props
                .mode_sync()
                .or_else(|| {
                    // If either async or plain then not sync.
                    field_props
                        .mode_async()
                        .as_ref()
                        .or(field_props.mode_plain().as_ref())
                        .not()
                })
                .or_else(|| self.arg_props().mode_sync())
                .unwrap_or_else(|| FXProp::new(false, *self.field_props.field().fieldx_attr_span()))
        })
    }

    pub(crate) fn mode_async(&self) -> FXProp<bool> {
        *self.mode_async.get_or_init(|| {
            let field_props = self.field_props();
            field_props
                .mode_async()
                .or_else(|| {
                    // If either sync or plain then not async.
                    field_props
                        .mode_sync()
                        .as_ref()
                        .or(field_props.mode_plain().as_ref())
                        .not()
                })
                .or_else(|| self.arg_props().mode_async())
                .unwrap_or_else(|| FXProp::new(false, *self.field_props.field().fieldx_attr_span()))
        })
    }

    pub(crate) fn mode_plain(&self) -> FXProp<bool> {
        *self.mode_plain.get_or_init(|| {
            let field_props = self.field_props();
            field_props
                .mode_plain()
                .or_else(|| {
                    // If either sync or async then not plain.
                    field_props
                        .mode_sync()
                        .as_ref()
                        .or(field_props.mode_async().as_ref())
                        .not()
                })
                .or_else(|| {
                    let arg_props = self.arg_props();
                    arg_props
                        .mode_plain()
                        .or_else(|| arg_props.mode_sync().as_ref().or(arg_props.mode_async().as_ref()).not())
                })
                .unwrap_or_else(|| FXProp::new(true, *self.field_props.field().fieldx_attr_span()))
        })
    }

    pub(crate) fn base_name(&self) -> &syn::Ident {
        self.base_name.get_or_init(|| {
            self.field_props()
                .base_name()
                .cloned()
                .unwrap_or_else(|| self.field_props.field().ident().unwrap())
        })
    }

    pub(crate) fn default_value(&self) -> Option<&syn::Expr> {
        self.field_props.default_value()
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde(&self) -> FXProp<bool> {
        *self.serde.get_or_init(|| {
            if let Some(field_serde) = self.field_props().serde() {
                FXProp::new(
                    field_serde.value().unwrap_or_else(|| *self.arg_props().serde()),
                    Some(field_serde.final_span()),
                )
            }
            else {
                self.arg_props().serde()
            }
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_optional(&self) -> FXProp<bool> {
        *self.serde_optional.get_or_init(|| self.optional().or(self.lazy()))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_default_value(&self) -> Option<&FXDefault> {
        self.field_props.serde_default_value()
    }

    #[cfg(feature = "serde")]
    #[inline(always)]
    pub(crate) fn serde_attributes(&self) -> Option<&FXAttributes> {
        self.field_props().serde_attributes()
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize(&self) -> FXProp<bool> {
        *self.serialize.get_or_init(|| {
            self.field_props()
                .serialize()
                .or_else(|| self.arg_props().serialize())
                .unwrap_or_else(|| FXProp::new(true, *self.field_props.field().fieldx_attr_span()))
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize(&self) -> FXProp<bool> {
        *self.deserialize.get_or_init(|| {
            self.field_props()
                .deserialize()
                .or_else(|| self.arg_props().deserialize())
                .unwrap_or_else(|| FXProp::new(true, *self.field_props.field().fieldx_attr_span()))
        })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_rename_serialize(&self) -> Option<&FXProp<String>> {
        self.serde_rename_serialize
            .get_or_init(|| {
                let field_props = self.field_props();
                field_props.serde_rename_serialize().cloned().or_else(|| {
                    field_props
                        .base_name()
                        .map(|bn| FXProp::new(bn.to_string(), Some(bn.span())))
                })
            })
            .as_ref()
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_rename_deserialize(&self) -> Option<&FXProp<String>> {
        self.serde_rename_deserialize
            .get_or_init(|| {
                let field_props = self.field_props();
                field_props.serde_rename_deserialize().cloned().or_else(|| {
                    field_props
                        .base_name()
                        .map(|bn| FXProp::new(bn.to_string(), Some(bn.span())))
                })
            })
            .as_ref()
    }
}
