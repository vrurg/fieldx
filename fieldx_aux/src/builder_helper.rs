//! Parameters of builder pattern and builder object.
use crate::{
    set_literals, FXAttributes, FXBool, FXHelperTrait, FXInto, FXNestingAttr, FXOrig, FXPubMode, FXPunctuated,
    FXString, FXSynValue, FXTriggerHelper, FromNestAttr,
};
use darling::{util::Flag, FromMeta};
use fieldx_derive_support::fxhelper;
use getset::Getters;
use syn::{Lit, Token};

// TODO try to issue warnings with `diagnostics` for sub-arguments which are not supported at struct or field level.
/// Implementation of builder argument.
#[fxhelper(validate = Self::validate)]
#[derive(Debug, Default, Getters)]
#[getset(get = "pub")]
pub struct FXBuilderHelper<const STRUCT: bool = false> {
    #[getset(skip)]
    attributes:      Option<FXAttributes>,
    #[getset(skip)]
    attributes_impl: Option<FXAttributes>,
    /// If set then builder setter methods must use the [`Into`] trait to coerce their arguments when possible.
    /// This should make both of the following allowed:
    ///
    /// ```ignore
    /// let f1 = Foo::builder().comment("comment 1").build()?;
    /// let f2 = Foo::builder().comment(String::from("comment 2")).build()?;
    /// ```
    #[getset(get = "pub")]
    into:            Option<FXBool>,
    /// Wether builder is required or optional. In `fieldx` it means that for `required` optional fields user must
    /// anyway always provide a value:
    ///
    /// ```ignore
    /// #[fxstruct(builder)]
    /// struct Foo {
    ///     #[fieldx(optional, builder(required))]
    ///     comment: String,
    /// }
    ///
    /// let foo = Foo::builder().build()?; // Error because the `comment` is left unset
    /// ```
    #[getset(get = "pub")]
    required:        Option<FXBool>,
    /// Means that by default a field doesn't get a builder unless explicitly specified. Only makes sense at struct
    /// level and when some builder parameters need to be set but we don't want all non-optional fields to get a builder
    /// method by default.
    opt_in:          Option<FXBool>,
    /// Name of the method that would be invoked right after builder constructs the object and before it's returned to
    /// the calling code.
    #[getset(get = "pub")]
    post_build:      Option<FXSynValue<syn::Ident, true>>,
    /// If we want a fallible `post_build` then this is where its error type is defined. If two path's are given then
    /// the second one must be a variant of the error enum that builder will use to report unset field.
    #[getset(get = "pub")]
    error:           Option<FXSynValue<FXPunctuated<syn::Path, Token![,], 1, 2>>>,
}

impl<const STRUCT: bool> FXBuilderHelper<STRUCT> {
    /// Shortcut to the `into` parameter.
    ///
    /// Since it makes sense at both struct and field level `Option` is returned to know exactly if it is set or not.
    pub fn is_into(&self) -> Option<bool> {
        self.into.as_ref().map(|i| i.is_true())
    }

    /// Shortcut to the `required` parameter.
    ///
    /// Since it makes sense at both struct and field level `Option` is returned to know exactly if it is set or not.
    pub fn is_required(&self) -> Option<bool> {
        self.required.as_ref().map(|r| r.is_true())
    }

    /// Shortcut to `opt_in` parameter.
    pub fn is_builder_opt_in(&self) -> bool {
        self.opt_in.as_ref().map_or(false, |o| o.is_true())
    }

    /// Shortcut to `post_build` parameter.
    pub fn has_post_build(&self) -> bool {
        self.post_build.is_some()
    }

    /// Accessor for `attributes_impl`.
    pub fn attributes_impl(&self) -> Option<&FXAttributes> {
        self.attributes_impl.as_ref()
    }

    /// The final error type.
    pub fn error_type(&self) -> Option<&syn::Path> {
        self.error().as_ref().and_then(|ev| ev.items().first())
    }

    /// The final error enum variant.
    pub fn error_variant(&self) -> Option<&syn::Path> {
        self.error().as_ref().and_then(|ev| ev.items().get(1))
    }

    #[doc(hidden)]
    pub fn validate(&self) -> darling::Result<()> {
        if !STRUCT {
            if self.error.is_some() {
                return Err(
                    darling::Error::custom(format!("parameter 'error' is only supported at struct level"))
                        .with_span(&self.error.fx_span()),
                );
            }
            if self.post_build.is_some() {
                return Err(darling::Error::custom(format!(
                    "parameter 'post_build' is only supported at struct level"
                ))
                .with_span(&self.post_build.fx_span()));
            }
            if self.opt_in.is_some() {
                return Err(
                    darling::Error::custom(format!("parameter 'opt_in' is only supported at struct level"))
                        .with_span(&self.opt_in.fx_span()),
                );
            }
        }
        Ok(())
    }
}

impl<const STRUCT: bool> FromNestAttr for FXBuilderHelper<STRUCT> {
    set_literals! {builder, ..1 => name as Lit::Str}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}
