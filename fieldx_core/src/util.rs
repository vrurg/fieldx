use fieldx_aux::FXBool;
pub use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
use fieldx_aux::FXProp;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSynValue;
use fieldx_aux::FXSyncMode;

#[macro_export]
macro_rules! simple_bool_prop {
    ($( $($ident:ident),+ );+ $(;)?) => {
        $(
            $crate::simple_bool_prop! { @fin $($ident),+ }
        )+
    };

    (@fin $field:ident, $prop_field:ident, $meth:ident) => {
        pub fn $meth(&self) -> Option<FXProp<bool>> {
            *self
                .$prop_field
                .get_or_init(|| self.source.$field.as_ref().map(|f| f.is_set()))
        }
    };
    (@fin $field:ident, $prop_field:ident $(,)?) => {
        pub fn $prop_field(&self) -> Option<FXProp<bool>> {
            *self
                .$prop_field
                .get_or_init(|| self.source.$field.as_ref().map(|f| f.into()))
        }
    };
    (@fin $field:ident) => {
        pub fn $field(&self) -> Option<FXProp<bool>> {
            *self
                .$field
                .get_or_init(|| self.source.$field.as_ref().map(|f| f.is_set()))
        }
    };
}

#[macro_export]
macro_rules! simple_type_prop {
    ( $( $field:ident, $type:ty, $prop_field:ident, $meth:ident );+ $(;)? ) => {
        $(
            pub fn $meth(&self) -> Option<&FXProp<$type>> {
                self.$prop_field
                    .get_or_init(|| {
                        self.source.$field.as_ref().map(|f: &FXNestingAttr<$type>| {
                            FXProp::new((**f).clone(), f.orig_span())
                        })
                    })
                    .as_ref()
            }
        )+
    };
    ( $( $field:ident, $type:ty, $prop_field:ident );+ $(;)? ) => {
        $(
            pub fn $prop_field(&self) -> Option<&FXProp<$type>> {
                self.$prop_field
                    .get_or_init(|| {
                        self.source.$field.as_ref().map(|f: &FXNestingAttr<$type>| {
                            FXProp::new((**f).clone(), f.orig_span())
                        })
                    })
                    .as_ref()
            }
        )+
    };
    ( $( $field:ident, $type:ty );+ $(;)? ) => {
        $(
            pub fn $field(&self) -> Option<&FXProp<$type>> {
                self.$field
                    .get_or_init(|| {
                        self.source.$field.as_ref().map(|f: &$crate::util::FXNestingAttr<$type>| {
                            FXProp::new((**f).clone(), f.orig_span())
                        })
                    })
                    .as_ref()
            }
        )+
    };
}

macro_rules! helper_standard_methods {
    ( $($helper:ident ),+ $(,)? ) => {
        $(
            $crate::simple_bool_prop!{ $helper }
            ::paste::paste! {
                pub fn [<$helper _visibility>](&self) -> Option<&syn::Visibility> {
                    self.[<$helper _visibility>]
                        .get_or_init(|| self.visibility_of(&self.source.$helper))
                        .as_ref()
                }

                pub fn [<$helper _ident>](&self) -> Option<&syn::Ident> {
                    self.[<$helper _ident>]
                        .get_or_init(||
                            self.source.$helper
                                .as_ref()
                                .and_then(
                                    |h| h.name()
                                         .map(|name| syn::Ident::new(&name, h.final_span())))
                        )
                        .as_ref()
                }

                pub fn [<$helper _attributes_fn>](&self) -> Option<&FXAttributes> {
                    self.source.$helper.as_ref().and_then(|h| h.attributes_fn())
                }
            }
        )+
    };
}

// Implement methods that are common for argument properties and field properties.
// Since the methods directly access the structs’ fields, implementing these via a trait makes little sense.
macro_rules! common_prop_impl {
    ( $( $std_helper:ident ),+ $(,)? ) => {
        $crate::simple_bool_prop! {
            inner_mut;
            into, into, is_into;
        }

        $crate::util::helper_standard_methods! {
            $( $std_helper ),+
        }

        pub fn accessor_mode(&self) -> Option<&FXProp<FXAccessorMode>> {
            self.accessor_mode
                .get_or_init(|| {
                    self.source
                        .accessor()
                        .as_ref()
                        .and_then(|am| am.mode())
                        .or_else(|| {
                            if *self.source.get_clone().is_true() {
                                Some(FXProp::new(
                                    FXAccessorMode::Clone,
                                    self.source.get_clone().as_ref().map(|c| c.final_span()),
                                ))
                            }
                            else if *self.source.get_copy().is_true() {
                                // Changed from self.source.get_copy().is_true()
                                Some(FXProp::new(
                                    FXAccessorMode::Copy,
                                    self.source.get_copy().as_ref().map(|c| c.final_span()),
                                ))
                            }
                            else {
                                None
                            }
                        })
                })
                .as_ref()
        }

        pub fn builder_into(&self) -> Option<FXProp<bool>> {
            *self.builder_into.get_or_init(|| {
                self.source
                    .builder
                    .as_ref()
                    .and_then(|b| b.is_into().into())
                    .or_else(|| self.is_into())
            })
        }

        pub fn builder_required(&self) -> Option<FXProp<bool>> {
            *self
                .builder_required
                .get_or_init(|| self.source.builder.as_ref().and_then(|b| b.is_required().into()))
        }

        pub fn setter_into(&self) -> Option<FXProp<bool>> {
            *self.setter_into.get_or_init(|| {
                self.source
                    .setter
                    .as_ref()
                    .and_then(|s| s.is_into().into())
                    .or_else(|| self.is_into().into())
            })
        }

        pub fn mode_sync(&self) -> Option<FXProp<bool>> {
            *self
                .mode_sync
                .get_or_init(|| mode_sync_prop(&self.source.mode_sync, &self.source.mode))
        }

        pub fn mode_async(&self) -> Option<FXProp<bool>> {
            *self
                .mode_async
                .get_or_init(|| mode_async_prop(&self.source.mode_async, &self.source.mode))
        }

        pub fn mode_plain(&self) -> Option<FXProp<bool>> {
            *self.mode_plain.get_or_init(|| mode_plain_prop(&self.source.mode))
        }

        pub fn lock(&self) -> Option<FXProp<bool>> {
            *self.lock.get_or_init(|| {
                self.source.lock.as_ref().map(|l| l.is_set()).or_else(|| {
                    self.mode_sync().and_then(|s| {
                        if *s && self.inner_mut().map_or(false, |i| *i) {
                            self.inner_mut()
                        }
                        else {
                            None
                        }
                    })
                })
            })
        }

        pub fn optional(&self) -> Option<FXProp<bool>> {
            *self.optional.get_or_init(|| {
                self.source.optional.as_ref().map(|o| o.is_set()).or_else(|| {
                    self.lazy().and_then(|l| {
                        if *l {
                            // If lazy is set then optional is false. Because lazy is explicit, its span is used.
                            Some(FXProp::new(false, l.orig_span()))
                        }
                        else {
                            None
                        }
                    })
                })
            })
        }

        pub fn helper_visibility(&self, helper_kind: FXHelperKind) -> Option<&syn::Visibility> {
            match helper_kind {
                FXHelperKind::Accessor => self.accessor_visibility(),
                FXHelperKind::AccessorMut => self.accessor_mut_visibility(),
                FXHelperKind::Builder => self.builder_visibility(),
                FXHelperKind::Clearer => self.clearer_visibility(),
                FXHelperKind::Lazy => self.lazy_visibility(),
                FXHelperKind::Predicate => self.predicate_visibility(),
                FXHelperKind::Reader => self.reader_visibility(),
                FXHelperKind::Setter => self.setter_visibility(),
                FXHelperKind::Writer => self.writer_visibility(),
            }
        }

        pub fn helper_attributes_fn(&self, helper_kind: FXHelperKind) -> Option<&FXAttributes> {
            match helper_kind {
                FXHelperKind::Accessor => self.accessor_attributes_fn(),
                FXHelperKind::AccessorMut => self.accessor_mut_attributes_fn(),
                FXHelperKind::Builder => self.builder_attributes_fn(),
                FXHelperKind::Clearer => self.clearer_attributes_fn(),
                FXHelperKind::Lazy => self.lazy_attributes_fn(),
                FXHelperKind::Predicate => self.predicate_attributes_fn(),
                FXHelperKind::Reader => self.reader_attributes_fn(),
                FXHelperKind::Setter => self.setter_attributes_fn(),
                FXHelperKind::Writer => self.writer_attributes_fn(),
            }
        }

        pub fn visibility_of<H>(&self, helper: &Option<fieldx_aux::FXNestingAttr<H>>) -> Option<syn::Visibility>
        where
            H: fieldx_aux::FXHelperTrait + fieldx_aux::FromNestAttr,
        {
            helper
                .as_ref()
                .and_then(|h| h.visibility().cloned())
                .or_else(|| self.visibility().cloned())
        }

        #[cfg(feature = "serde")]
        pub fn serde_attributes(&self) -> Option<&FXAttributes> {
            self.source.serde().as_ref().and_then(|s| s.attributes().as_ref())
        }

        #[cfg(feature = "serde")]
        pub fn serde_default_value(&self) -> Option<&FXDefault> {
            self.serde_default_value
                .get_or_init(|| {
                    self.source
                        .serde()
                        .as_ref()
                        .and_then(|s| s.default_value())
                        .cloned()
                })
                .as_ref()
        }

        #[cfg(feature = "serde")]
        pub fn serde_rename_serialize(&self) -> Option<&FXProp<String>> {
            self.serde_rename_serialize
                .get_or_init(|| {
                    self.source
                        .serde()
                        .as_ref()
                        .and_then(|s| {
                            let r = s.rename().as_ref();
                            r
                        })
                        .and_then(|r| r.serialize())
                })
                .as_ref()
        }

        #[cfg(feature = "serde")]
        pub fn serde_rename_deserialize(&self) -> Option<&FXProp<String>> {
            self.serde_rename_deserialize
                .get_or_init(|| {
                    self.source
                        .serde()
                        .as_ref()
                        .and_then(|s| s.rename().as_ref())
                        .and_then(|r| r.deserialize().clone())
                })
                .as_ref()
        }

        #[cfg(feature = "serde")]
        pub fn serde_forward_attrs(&self) -> Option<&HashSet<syn::Path>> {
            self.serde_forward_attrs
                .get_or_init(|| {
                    self.source.serde.as_ref().and_then(|s| {
                        s.forward_attrs()
                            .as_ref()
                            .map(|fa| fa.value().iter().cloned().collect::<HashSet<syn::Path>>())
                    })
                })
                .as_ref()
        }
    };
}

#[macro_export]
macro_rules! doc_props {
    ($($doc_prop:ident from $arg:ident . $subarg:ident );+ $(;)?) => {
        $(
            pub fn $doc_prop(&self) -> Option<&FXProp<Vec<syn::LitStr>>> {
                self.$doc_prop
                    .get_or_init(|| {
                        self.source
                            .$arg()
                            .as_ref()
                            .and_then(|p| p.$subarg().and_then(|doc| doc.into()))
                    })
                    .as_ref()
            }
        )+
    };
}

pub(crate) use common_prop_impl;
pub(crate) use helper_standard_methods;

#[allow(dead_code)]
pub fn feature_required<T, O>(feature: &str, arg: &Option<T>) -> Option<darling::Error>
where
    T: FXSetState + FXOrig<O>,
    O: syn::spanned::Spanned,
{
    if let Some(arg) = arg {
        if *arg.is_set() {
            return Some(
                darling::Error::custom(format!("feature '{feature}' is required")).with_span(&arg.final_span()),
            );
        }
    }
    None
}

pub fn mode_sync_prop(mode_sync: &Option<FXBool>, mode: &Option<FXSynValue<FXSyncMode>>) -> Option<FXProp<bool>> {
    mode_sync
        .as_ref()
        .map(|th| th.is_set())
        .or_else(|| mode.as_ref().map(|m| FXProp::new(m.is_sync(), m.orig_span())))
}

pub fn mode_async_prop(mode_async: &Option<FXBool>, mode: &Option<FXSynValue<FXSyncMode>>) -> Option<FXProp<bool>> {
    mode_async
        .as_ref()
        .map(|th| th.is_set())
        .or_else(|| mode.as_ref().map(|m| FXProp::new(m.is_async(), m.orig_span())))
}

pub fn mode_plain_prop(mode: &Option<FXSynValue<FXSyncMode>>) -> Option<FXProp<bool>> {
    mode.as_ref().map(|m| FXProp::new(m.is_plain(), m.orig_span()))
}
