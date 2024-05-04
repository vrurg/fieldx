use super::{FXHelper, FXHelperTrait, FXNestingAttr, FromNestAttr};
use crate::util::set_literals;
use darling::{
    util::{Flag, PathList},
    FromMeta,
};
use getset::Getters;
use syn::Lit;

#[derive(FromMeta, Clone, Default, Debug, Getters)]
#[darling(default)]
pub(crate) struct FXSerdeDefault {
    off:   Flag,
    #[getset(get="pub")]
    value: Option<String>,
}

#[derive(Default, Debug, Getters, FromMeta, Clone)]
#[getset(get = "pub(crate)")]
pub(crate) struct FXSerdeHelper<const SHADOW_NAME: bool = false> {
    off:           Flag,
    serialize:     Option<FXHelper<true>>,
    deserialize:   Option<FXHelper<true>>,
    // Attributes of the original struct to be used with the shadow struct.
    forward_attrs: Option<PathList>,
    #[darling(rename = "default")]
    default_value: Option<FXNestingAttr<FXSerdeDefault>>,
    // Name of the new type to be used for deserialization. Normally its <ident>Shadow
    shadow_name:   Option<String>,
}

impl FromNestAttr for FXSerdeHelper {
    set_literals! {serde}

    fn for_keyword() -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl FXSerdeHelper {
    pub(crate) fn is_true(&self) -> bool {
        !self.off.is_present()
    }

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

    // pub(crate) fn default_value(&self) -> Option<&String> {
    //     self.default_value.as_ref().and_then(|d| d.value().as_ref())
    // }
}

impl FromNestAttr for FXSerdeDefault {
    set_literals! {default, ..1 => value as Lit::Str}

    fn for_keyword() -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl FXSerdeDefault {
    fn is_true(&self) -> bool {
        !self.off.is_present()
    }
}