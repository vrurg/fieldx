use crate::{
    set_literals, FXAttributes, FXBool, FXBoolHelper, FXDefault, FXInto, FXNestingAttr, FXOrig, FXProp, FXSetState,
    FXString, FXSynValue, FXTriggerHelper, FXTryFrom, FXTryInto, FromNestAttr,
};
use darling::{
    util::{Flag, PathList},
    FromMeta,
};
use getset::Getters;
use syn::Lit;

#[derive(Default, Debug, FromMeta, Clone)]
pub struct FXSerdeRename {
    serialize:   Option<FXSynValue<syn::LitStr>>,
    deserialize: Option<FXSynValue<syn::LitStr>>,
}

impl FXSerdeRename {
    pub fn serialize(&self) -> Option<FXProp<String>> {
        self.serialize.as_ref().and_then(|s| s.into())
    }

    pub fn deserialize(&self) -> Option<FXProp<String>> {
        self.deserialize.as_ref().and_then(|d| d.into())
    }
}

impl FXTryFrom<syn::Lit> for FXSerdeRename {
    type Error = darling::Error;

    fn fx_try_from(value: syn::Lit) -> Result<Self, Self::Error> {
        match value {
            syn::Lit::Str(s) => Ok(Self {
                serialize:   Some(s.clone().fx_into()),
                deserialize: Some(s.fx_into()),
            }),
            _ => Err(darling::Error::unexpected_lit_type(&value)),
        }
    }
}

impl FXTryFrom<&syn::Lit> for FXSerdeRename {
    type Error = darling::Error;

    fn fx_try_from(value: &syn::Lit) -> Result<Self, Self::Error> {
        match value {
            syn::Lit::Str(s) => Ok(Self {
                serialize:   Some(s.clone().fx_into()),
                deserialize: Some(s.clone().fx_into()),
            }),
            _ => Err(darling::Error::unexpected_lit_type(value)),
        }
    }
}

impl FromNestAttr for FXSerdeRename {
    fn set_literals(self, literals: &Vec<Lit>) -> darling::Result<Self> {
        if literals.len() > 1 {
            return Err(darling::Error::too_many_items(1));
        }
        else if literals.len() == 0 {
            return Err(darling::Error::custom("Expected a single string literal argument"));
        }

        Ok((&literals[0]).fx_try_into()?)
    }
}

#[derive(Default, Debug, Getters, FromMeta, Clone)]
#[getset(get = "pub")]
#[darling(and_then = Self::validate)]
pub struct FXSerdeHelper {
    off:           Flag,
    attributes:    Option<FXAttributes>,
    serialize:     Option<FXBool>,
    deserialize:   Option<FXBool>,
    #[getset(skip)]
    #[darling(rename = "vis")]
    visibility:    Option<FXSynValue<syn::Visibility>>,
    private:       Option<FXBool>,
    // Attributes of the original struct to be used with the shadow struct.
    forward_attrs: Option<PathList>,
    #[darling(rename = "default")]
    #[getset(skip)]
    default_value: Option<FXDefault>,
    // Name of the new type to be used for deserialization. By default it's __<ident>Shadow
    shadow_name:   Option<FXString>,
    rename:        Option<FXNestingAttr<FXSerdeRename>>,
}

impl FromNestAttr for FXSerdeHelper {
    set_literals! {serde, .. 1 => rename as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl FXTriggerHelper for FXSerdeHelper {
    fn is_true(&self) -> FXProp<bool> {
        if self.off.is_present() {
            FXProp::new(false, Some(self.off.span()))
        }
        else {
            FXProp::new(true, None)
        }
    }
}

impl FXSetState for FXSerdeHelper {
    fn is_set(&self) -> FXProp<bool> {
        if self.off.is_present() {
            FXProp::new(false, Some(self.off.span()))
        }
        else {
            // If `is_serde` returns `None`, then it means that `serialize` and `deserialize` are not explicitly set.
            // Therefore, the state is considered set because this implies that both serialization and deserialization
            // are enabled.
            let is_serde = self.is_serde();
            FXProp::new(is_serde.value().unwrap_or(true), is_serde.orig_span())
        }
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

    pub fn is_serde(&self) -> FXProp<Option<bool>> {
        // Consider as Some(true) if not `serde(off)` or any of `serialize` or `deserialize` is defined and not both are
        // `off`. I.e. since `serde(deserialize(off))` implies `serialize` being `on` then the outcome is `Some(true)`.
        let is_true = self.is_true();
        if *is_true {
            let is_serialize: Option<FXProp<bool>> = self.serialize.as_ref().map(|s| s.into());
            let is_deserialize: Option<FXProp<bool>> = self.deserialize.as_ref().map(|d| d.into());

            if is_serialize.is_none() && is_deserialize.is_none() {
                FXProp::new(None, None)
            }
            else if is_serialize.is_some()
                && is_deserialize.is_some()
                && !(*is_serialize.unwrap() || *is_deserialize.unwrap())
            {
                FXProp::new(Some(false), None)
            }
            else {
                FXProp::new(Some(true), None)
                // is_serialize
                //     .or(is_deserialize)
                //     .map(|is| FXProp::new(Some(*is), is.orig_span()))
                //     .unwrap()
            }
        }
        else {
            FXProp::new(Some(false), is_true.orig_span())
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
        if *self.private.is_true() {
            return Some(&syn::Visibility::Inherited);
        }
        self.visibility.as_ref().map(|v| v.as_ref())
    }
}
