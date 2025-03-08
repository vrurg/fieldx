use crate::{
    set_literals, validate_exclusives, FXAttributes, FXBool, FXDefault, FXInto, FXNestingAttr, FXOrig, FXProp,
    FXPropBool, FXPubMode, FXString, FXSynValue, FXTriggerHelper, FromNestAttr,
};
use darling::{
    util::{Flag, PathList},
    FromMeta,
};
use getset::Getters;
use proc_macro2::Span;
use syn::Lit;

#[derive(Default, Debug, Getters, FromMeta, Clone)]
#[getset(get = "pub")]
#[darling(and_then = Self::validate)]
pub struct FXSerdeHelper {
    off:           Flag,
    attributes:    Option<FXAttributes>,
    serialize:     Option<FXBool>,
    deserialize:   Option<FXBool>,
    #[getset(skip)]
    visibility:    Option<FXSynValue<syn::Visibility>>,
    // Attributes of the original struct to be used with the shadow struct.
    forward_attrs: Option<PathList>,
    #[darling(rename = "default")]
    #[getset(skip)]
    default_value: Option<FXDefault>,
    // Name of the new type to be used for deserialization. By default it's __<ident>Shadow
    shadow_name:   Option<FXString>,
}

impl FromNestAttr for FXSerdeHelper {
    set_literals! {serde, .. 1 => shadow_name as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl FXTriggerHelper for FXSerdeHelper {
    fn is_true(&self) -> FXProp<bool> {
        FXProp::from(self.off).not()
    }
}

impl FXSerdeHelper {
    fn validate(self) -> darling::Result<Self> {
        // self.validate_exclusives()
        //     .map_err(|err| err.with_span(&Span::call_site()))?;
        Ok(self)
    }

    // Some(true) only if `serialize` is explicitly set to `true`
    // Some(false) only if explicitly disabled or `deserialize` is explicitly set to `true`
    pub fn needs_serialize(&self) -> Option<FXProp<bool>> {
        self.serialize.as_ref().map(|s| s.into()).or_else(|| {
            self.deserialize.as_ref().and_then(|d| {
                if *d.is_true() {
                    Some(FXProp::new(false, d.orig_span()))
                }
                else {
                    None
                }
            })
        })
    }

    // Some(true) only if `deserialize` is explicitly set to `true`
    // Some(false) only if explicitly disabled or `serialize` is explicitly set to `true`
    pub fn needs_deserialize(&self) -> Option<FXProp<bool>> {
        self.deserialize.as_ref().map(|d| d.into()).or_else(|| {
            self.serialize
                .as_ref()
                .map(|s| FXProp::new(!*s.is_true(), s.orig_span()))
        })
    }

    /// `span` provides the span to use when neither `serialize`, `deserialize`, nor `off` is explicitly set.
    pub fn is_serde(&self, default_span: Option<Span>) -> Option<FXProp<bool>> {
        // Consider as Some(true) if not `serde(off)` or any of `serialize` or `deserialize` is defined and not both are
        // `off`. I.e. since `serde(deserialize(off))` implies `serialize` being `on` then the outcome is `Some(true)`.
        if *self.is_true() {
            let is_serialize: Option<FXProp<bool>> = self.serialize.as_ref().map(|s| s.into());
            let is_deserialize: Option<FXProp<bool>> = self.deserialize.as_ref().map(|d| d.into());

            if is_serialize.is_none() && is_deserialize.is_none() {
                None
            }
            else if is_serialize.is_some()
                && is_deserialize.is_some()
                && !(*is_serialize.unwrap() || *is_deserialize.unwrap())
            {
                Some(FXProp::new(false, default_span))
            }
            else if is_serialize.is_some() && *is_serialize.unwrap() {
                is_serialize
            }
            else {
                is_deserialize
            }
        }
        else {
            Some(FXProp::new(false, Some(self.off.span())))
        }
    }

    #[inline]
    pub fn accepts_attr(&self, attr: &syn::Attribute) -> bool {
        self.forward_attrs.as_ref().map_or(true, |fa| fa.contains(attr.path()))
    }

    #[inline]
    pub fn has_default(&self) -> bool {
        self.default_value.as_ref().map_or(false, |d| *d.is_true())
    }

    #[inline]
    pub fn default_value(&self) -> Option<&FXDefault> {
        self.default_value.as_ref()
    }

    #[inline]
    pub fn visibility(&self) -> Option<&syn::Visibility> {
        self.visibility.as_ref().map(|v| v.as_ref())
    }
}
