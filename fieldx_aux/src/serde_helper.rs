use crate::join_token_list;
use crate::set_literals;
use crate::to_tokens_vec;
use crate::FXAttributes;
use crate::FXBool;
use crate::FXDefault;
use crate::FXDoc;
use crate::FXNestingAttr;
use crate::FXOrig;
use crate::FXProp;
use crate::FXPropBool;
use crate::FXPunctuated;
use crate::FXSetState;
use crate::FXString;
use crate::FXSynValue;
use crate::FXTrigger;
use crate::FXTryFrom;
use crate::FXTryInto;
use crate::FromNestAttr;

use darling::util::Flag;
use darling::FromMeta;
use getset::Getters;
use quote::quote_spanned;
use quote::ToTokens;
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
                serialize:   ("serialize", s.clone()).fx_try_into()?,
                deserialize: ("deserialize", s).fx_try_into()?,
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
                serialize:   ("serialize", s.clone()).fx_try_into()?,
                deserialize: ("deserialize", s.clone()).fx_try_into()?,
            }),
            _ => Err(darling::Error::unexpected_lit_type(value)),
        }
    }
}

impl FromNestAttr for FXSerdeRename {
    fn set_literals(self, literals: &[Lit]) -> darling::Result<Self> {
        if literals.len() > 1 {
            return Err(darling::Error::too_many_items(1));
        }
        else if literals.is_empty() {
            return Err(darling::Error::custom("Expected a single string literal argument"));
        }

        (&literals[0]).fx_try_into()
    }
}

impl ToTokens for FXSerdeRename {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut toks = vec![];
        if let Some(ref serialize) = self.serialize {
            toks.push(quote_spanned![serialize.final_span()=> #serialize]);
        }
        if let Some(ref deserialize) = self.deserialize {
            toks.push(quote_spanned![deserialize.final_span()=> #deserialize]);
        }
        tokens.extend(join_token_list!(toks));
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
    #[getset(skip)]
    private:       Option<FXBool>,
    // Attributes of the original struct to be used with the shadow struct.
    forward_attrs: Option<FXSynValue<FXPunctuated<syn::Path, syn::Token![,]>>>,
    #[darling(rename = "default")]
    #[getset(skip)]
    default_value: Option<FXDefault>,
    // Name of the new type to be used for deserialization. By default it's __<ident>Shadow
    #[getset(skip)]
    shadow_name:   Option<FXString>,
    rename:        Option<FXNestingAttr<FXSerdeRename>>,
    #[getset(skip)]
    doc:           Option<FXDoc>,
}

impl FromNestAttr for FXSerdeHelper {
    set_literals! {serde, .. 1 => rename}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

impl FXSetState for FXSerdeHelper {
    fn is_set(&self) -> FXProp<bool> {
        if self.off.is_present() {
            FXProp::from(&self.off).not()
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
        self.serialize.as_ref().map(|s| s.is_set()).or_else(|| {
            self.deserialize.as_ref().and_then(|d| {
                if *d.is_set() {
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
        self.deserialize.as_ref().map(|d| d.is_set()).or_else(|| {
            self.serialize
                .as_ref()
                .map(|s| FXProp::new(!*s.is_set(), s.orig_span()))
        })
    }

    pub fn is_serde(&self) -> FXProp<Option<bool>> {
        // Consider as Some(true) if not `serde(off)` or any of `serialize` or `deserialize` is defined and not both are
        // `off`. I.e. since `serde(deserialize(off))` implies `serialize` being `on` then the outcome is `Some(true)`.
        let is_true = FXProp::from(&self.off).not();
        if *is_true {
            let is_serialize: Option<FXProp<bool>> = self.serialize.as_ref().map(|s| s.is_set());
            let is_deserialize: Option<FXProp<bool>> = self.deserialize.as_ref().map(|d| d.is_set());

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
            }
        }
        else {
            FXProp::new(Some(false), is_true.orig_span())
        }
    }

    #[inline]
    pub fn accepts_attr(&self, attr: &syn::Attribute) -> bool {
        self.forward_attrs
            .as_ref()
            .map_or(true, |fa| fa.iter().any(|p| p == attr.path()))
    }

    #[inline]
    pub fn has_default(&self) -> bool {
        self.default_value.as_ref().is_some_and(|d| *d.is_set())
    }

    #[inline]
    pub fn default_value(&self) -> Option<&FXDefault> {
        self.default_value.as_ref()
    }

    #[inline]
    pub fn doc(&self) -> Option<&FXDoc> {
        self.doc.as_ref()
    }

    #[inline]
    pub fn visibility(&self) -> Option<&syn::Visibility> {
        if *self.private.is_true() {
            return Some(&syn::Visibility::Inherited);
        }
        self.visibility.as_ref().map(|v| v.as_ref())
    }

    /// Give the full property of the visibility for cases where detailed analysis is needed.
    #[inline]
    pub fn orig_visibility(&self) -> Option<&FXSynValue<syn::Visibility>> {
        self.visibility.as_ref()
    }

    #[inline]
    pub fn shadow_name(&self) -> Option<&FXString> {
        self.shadow_name.as_ref()
    }

    #[inline]
    pub fn private(&self) -> Option<&FXBool> {
        self.private.as_ref()
    }
}

impl ToTokens for FXSerdeHelper {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut toks = vec![];
        if self.off.is_present() {
            toks.push(quote_spanned![self.off.span()=> off]);
        }

        toks.extend(to_tokens_vec!(self:
            attributes,
            serialize,
            deserialize,
            doc,
            forward_attrs,
            private,
            rename,
            shadow_name,
            visibility
        ));

        if let Some(ref default_value) = self.default_value {
            toks.push(quote_spanned![default_value.final_span()=> default( #default_value )]);
        }

        tokens.extend(join_token_list!(toks));
    }
}

#[cfg(test)]
mod tests {
    use darling::FromMeta;
    use darling::ToTokens;
    use quote::quote;

    use crate::FXSerdeHelper;

    #[test]
    fn test_roundtrip() {
        let input: syn::Meta = syn::parse2(quote! {
            serde(
                off,
                serialize(off),
                deserialize,
                vis(pub(crate)),
                default(Self::serde_default()),
                shadow_name("FooDup"),
                forward_attrs(a1, a2, allow),
                rename("new_name"),
                doc("line1", "line2")
            )
        })
        .unwrap();
        let helper = FXSerdeHelper::from_meta(&input).unwrap();
        let expected = quote! {
            off,
            serialize(off),
            deserialize(),
            doc("line1", "line2"),
            forward_attrs(a1, a2, allow),
            rename(serialize("new_name"), deserialize("new_name")),
            shadow_name("FooDup"),
            vis(pub(crate)),
            default(Self::serde_default())
        };

        assert_eq!(helper.to_token_stream().to_string(), expected.to_string());
    }
}
