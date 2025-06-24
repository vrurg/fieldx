//! Parameters of builder pattern and builder object.
use crate::set_literals;
use crate::validate_no_subarg_at_level;
use crate::FXAttributes;
use crate::FXBool;
use crate::FXDoc;
use crate::FXOrig;
use crate::FXProp;
use crate::FXPropBool;
use crate::FXPunctuated;
use crate::FXSetState;
use crate::FXString;
use crate::FXSynValue;
use crate::FXTryInto;
use crate::FromNestAttr;

use darling::util::Flag;
use darling::FromMeta;
use fieldx_derive_support::fxhelper;
use getset::Getters;
use proc_macro2::TokenStream;
use syn::Token;

// TODO try to issue warnings with `diagnostics` for sub-arguments which are not supported at struct or field level.
/// Implementation of builder argument.
#[fxhelper(validate = Self::validate, to_tokens)]
#[derive(Debug, Default)]
pub struct FXBuilderHelper<const STRUCT: bool = false> {
    #[getset(skip)]
    attributes:      Option<FXAttributes>,
    #[getset(skip)]
    attributes_impl: Option<FXAttributes>,

    /// If true (default), the builder type will implement the `Default` trait.
    #[darling(rename = "default")]
    #[getset(skip)]
    needs_default: Option<FXBool>,

    /// If set then builder setter methods must use the [`Into`] trait to coerce their arguments when possible.
    /// This should make both of the following allowed:
    ///
    /// ```ignore
    /// let f1 = Foo::builder().comment("comment 1").build()?;
    /// let f2 = Foo::builder().comment(String::from("comment 2")).build()?;
    /// ```
    #[getset(get = "pub")]
    into: Option<FXBool>,

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
    required: Option<FXBool>,

    /// Means that by default a field doesn't get a builder unless explicitly specified. Only makes sense at struct
    /// level and when some builder parameters need to be set but we don't want all non-optional fields to get a builder
    /// method by default.
    opt_in: Option<FXBool>,

    /// Name of the method that would be invoked right after builder constructs the object and before it's returned to
    /// the calling code.
    post_build: Option<FXSynValue<syn::Ident, true>>,

    /// If we want a fallible `post_build` then this is where its error type is defined. If two path's are given then
    /// the second one must be a variant of the error enum that builder will use to report unset field.
    #[getset(get = "pub")]
    error: Option<FXSynValue<FXPunctuated<syn::Path, Token![,], 1, 2>>>,

    /// Prefix for the builder setter methods.
    prefix: Option<FXString>,

    /// Documentation for the builder method.
    method_doc: Option<FXDoc>,
}

impl<const STRUCT: bool> FXBuilderHelper<STRUCT> {
    pub fn needs_default(&self) -> FXProp<bool> {
        self.needs_default
            .as_ref()
            .map_or_else(|| FXProp::new(true, None), |nd| nd.is_set())
    }

    /// Shortcut to the `into` parameter.
    ///
    /// Since it makes sense at both struct and field level `Option` is returned to know exactly if it is set or not.
    #[inline]
    pub fn is_into(&self) -> Option<FXProp<bool>> {
        self.into.as_ref().map(|i| i.is_set())
    }

    /// Shortcut to the `required` parameter.
    ///
    /// Since it makes sense at both struct and field level `Option` is returned to know exactly if it is set or not.
    #[inline]
    pub fn is_required(&self) -> Option<FXProp<bool>> {
        self.required.as_ref().map(|r| r.is_set())
    }

    /// Shortcut to `post_build` parameter.
    pub fn has_post_build(&self) -> FXProp<bool> {
        self.post_build
            .as_ref()
            .map_or_else(|| FXProp::new(false, None), |pb| FXProp::new(true, pb.orig_span()))
    }

    /// Accessor for `attributes_impl`.
    #[inline]
    pub fn attributes(&self) -> Option<&FXAttributes> {
        self.attributes.as_ref()
    }

    /// Accessor for `attributes_impl`.
    #[inline]
    pub fn attributes_impl(&self) -> Option<&FXAttributes> {
        self.attributes_impl.as_ref()
    }

    /// The final error type.
    pub fn error_type(&self) -> Option<&syn::Path> {
        self.error().as_ref().and_then(|ev| ev.first())
    }

    /// The final error enum variant.
    #[inline]
    pub fn error_variant(&self) -> Option<&syn::Path> {
        self.error().as_ref().and_then(|ev| ev.get(1))
    }

    #[inline]
    pub fn method_doc(&self) -> Option<&FXDoc> {
        self.method_doc.as_ref()
    }

    #[inline]
    pub fn post_build(&self) -> Option<&FXSynValue<syn::Ident, true>> {
        self.post_build.as_ref()
    }

    #[inline]
    pub fn prefix(&self) -> Option<&FXString> {
        self.prefix.as_ref()
    }

    #[inline]
    pub fn opt_in(&self) -> Option<&FXBool> {
        self.opt_in.as_ref()
    }

    #[doc(hidden)]
    pub fn validate(&self) -> darling::Result<()> {
        let mut acc = darling::Error::accumulator();
        if !STRUCT {
            validate_no_subarg_at_level!( self, "builder", "field", acc: error, post_build, opt_in, prefix );
        }
        acc.finish()?;
        Ok(())
    }
}

impl<const STRUCT: bool> FromNestAttr for FXBuilderHelper<STRUCT> {
    set_literals! {builder, ..1 => name}

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use darling::FromMeta;
    use proc_macro2::TokenStream;
    use quote::quote;
    use quote::ToTokens;
    use syn::parse2;

    use crate::FXBuilderHelper;
    use crate::FXNestingAttr;

    #[test]
    fn test_roundtrip() {
        let input: TokenStream = quote! {
            builder(
                off,
                "TestBuilder",
                attributes( third_party(1,2,3) ),
                attributes_fn( deny(unused) ),
                attributes_impl( deny(unused) ),
                default,
                into,
                required,
                opt_in,
                post_build( adjust_struct ),
                error( std::io::Error, std::io::ErrorKind::Other ),
                prefix( "set_" ),
                vis(pub(crate)),
                doc("# Builder", "", "Test doc."),
                method_doc("# Builder Method", "", "Lorem ipsum.")
            )
        };
        let input: syn::Meta = parse2(input).unwrap();
        let helper = FXNestingAttr::<FXBuilderHelper<true>>::from_meta(&input).unwrap();

        let expected: TokenStream = quote! {
            builder(
                off,
                name( "TestBuilder" ),
                attributes_fn( deny(unused) ),
                vis(pub(crate)),
                doc("# Builder", "", "Test doc."),
                attributes( third_party(1,2,3) ),
                attributes_impl( deny(unused) ),
                default(),
                into(),
                required(),
                opt_in(),
                post_build( adjust_struct ),
                error( std::io::Error, std::io::ErrorKind::Other ),
                prefix( "set_" ),
                method_doc("# Builder Method", "", "Lorem ipsum.")
            )
        };

        assert_eq!(helper.to_token_stream().to_string(), expected.to_string());
    }
}
