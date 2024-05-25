use super::{FXBoolArg, FXDefault, FXInto, FXStringArg, FXTriggerHelper, FromNestAttr};
use crate::util::set_literals;
use darling::{
    ast::NestedMeta,
    util::{Flag, PathList},
    FromMeta,
};
use getset::Getters;
use syn::Lit;

#[derive(Default, Debug, Getters, FromMeta, Clone)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXSerdeHelper {
    off:           Flag,
    serialize:     Option<FXBoolArg>,
    deserialize:   Option<FXBoolArg>,
    // Attributes of the original struct to be used with the shadow struct.
    forward_attrs: Option<PathList>,
    #[darling(rename = "default")]
    #[getset(skip)]
    default_value: Option<FXDefault<true>>,
    // Name of the new type to be used for deserialization. By default it's __<ident>Shadow
    #[getset(skip)]
    shadow_name:   Option<FXStringArg>,
}

impl FromNestAttr for FXSerdeHelper {
    set_literals! {serde, .. 1 => shadow_name as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl FXTriggerHelper for FXSerdeHelper {
    fn is_true(&self) -> bool {
        !self.off.is_present()
    }
}

impl FXSerdeHelper {
    pub(crate) fn needs_serialize(&self) -> Option<bool> {
        self.serialize
            .as_ref()
            .map(|s| s.is_true())
            .or_else(|| self.deserialize.as_ref().map(|d| !d.is_true()))
    }

    pub(crate) fn needs_deserialize(&self) -> Option<bool> {
        self.deserialize
            .as_ref()
            .map(|d| d.is_true())
            .or_else(|| self.serialize.as_ref().map(|s| !s.is_true()))
    }

    pub(crate) fn is_serde(&self) -> Option<bool> {
        // Consider as Some(true) if not `serde(off)` or any of `serialize` or `deserialize` is defined and not both are
        // `off`. I.e. since `serde(deserialize(off))` implies `serialize` being `on` then the outcome is `Some(true)`.
        if self.is_true() {
            let is_serialize = self.serialize.as_ref().map(|s| s.is_true());
            let is_deserialize = self.deserialize.as_ref().map(|d| d.is_true());

            if is_serialize.is_none() && is_deserialize.is_none() {
                return None;
            }

            Some(is_serialize.unwrap_or(true) || is_deserialize.unwrap_or(true))
        }
        else {
            Some(false)
        }
    }

    pub(crate) fn accepts_attr(&self, attr: &syn::Attribute) -> bool {
        self.forward_attrs.as_ref().map_or(true, |fa| fa.contains(attr.path()))
    }

    pub(crate) fn has_default(&self) -> bool {
        self.default_value.as_ref().map_or(false, |d| d.is_true())
    }

    // pub(crate) fn has_default_value(&self) -> bool {
    //     self.default_value
    //         .as_ref()
    //         .map_or(false, |d| d.is_true() && d.value().is_some())
    // }

    pub(crate) fn default_value(&self) -> Option<&NestedMeta> {
        if self.has_default() {
            self.default_value.as_ref().and_then(|d| d.value().as_ref())
        }
        else {
            None
        }
    }

    pub(crate) fn default_value_raw(&self) -> Option<&FXDefault<true>> {
        self.default_value.as_ref()
    }

    pub(crate) fn shadow_name(&self) -> Option<&String> {
        self.shadow_name.as_ref().and_then(|sn| sn.value())
    }
}
