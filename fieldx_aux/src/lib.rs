#![doc(html_root_url = "https://docs.rs/fieldx_aux/0.2.0/")]
//! Helper crate for [`fieldx`] and any third-party crates that extend its functionality.
//!
//! `fieldx` is heavily based on the [`darling`] crate, which greatly simplifies proc-macro development,
//! but also imposes some constraints on attribute argument syntax. This crate overcomes these limitations
//! and provides support for attribute kinds required to implement `fieldx`.
//!
//! Here is a brief breakdown of what is provided:
//!
//! - Support for nested arguments, i.e. those that look like `arg1("value", trigger, subarg(...))`.
//! - Support for syntax elements not covered by the `darling` crate, such as
//!   `some_type(crate::types::Foo)` and
//!   `error(crate::error::Error, crate::error::Error::SomeProblem("with details"))`[^tuple].
//! - A set of types implementing standard `fieldx` arguments like helpers or literal values.
//!
//! [^tuple]: Here, the first argument of `error()`—`Error`—is an enum, and `SomeProblem` is one of its variants.
//!
//! # Usage
//!
//! Imagine we are implementing a field-level attribute `foo` using the [`darling::FromField`] trait, and we want it to
//! accept the following arguments:
//!
//! - `trigger`: enables or disables certain functionality
//! - `action`: specifies a method with special meaning
//! - `comment`: accepts arbitrary text
//! - `vis`: indicates whether field-related code should be public, and if so, which kind of `pub` modifier to use
//!
//! A field declaration may take the following form with the attribute:
//!
//! ```ignore
//!     #[foo(
//!         trigger,
//!         action("method_name", private),
//!         comment("Whatever we consider useful."),
//!         vis(pub(crate))
//!     )]
//!     bar: usize,
//! ```
//!
//! For this, you'll need the following declaration somewhere in your proc-macro implementation:
//!
//! ```ignore
//! #derive(FromField)
//! #[darling(attributes(foo))]
//! struct FooField {
//!     // ... skipping some darling default fields ...
//!
//!     trigger: Option<FXBool>,
//!     action: Option<FXHelper>,
//!     comment: Option<FXString>,
//!     vis: Option<FXSynValue<syn::Visibility>>,
//! }
//! ```
//!
//! That's all; this crate will take care of implementing the arguments for you!
//!
//! [`fieldx`]: https://docs.rs/fieldx
//! [`darling`]: https://docs.rs/darling

pub mod accessor_helper;
pub mod attributes;
pub mod base_helper;
pub mod builder_helper;
pub mod default_arg;
pub mod doc_arg;
pub mod fallible;
pub mod nesting_attr;
pub mod property;
pub mod serde_helper;
pub mod setter_helper;
pub mod syn_value;
pub mod traits;
#[doc(hidden)]
pub mod util;
pub mod value;
pub mod with_origin;

pub use crate::accessor_helper::FXAccessorHelper;
pub use crate::accessor_helper::FXAccessorMode;
pub use crate::attributes::FXAttribute;
pub use crate::base_helper::FXBaseHelper;
pub use crate::builder_helper::FXBuilderHelper;
pub use crate::default_arg::FXDefault;
pub use crate::doc_arg::FXDocArg;
pub use crate::fallible::FXFallible;
pub use crate::nesting_attr::FXNestingAttr;
pub use crate::nesting_attr::FromNestAttr;
pub use crate::property::*;
pub use crate::serde_helper::FXSerdeHelper;
pub use crate::setter_helper::FXSetterHelper;
pub use crate::syn_value::FXPunctuated;
pub use crate::syn_value::FXSynTupleArg;
pub use crate::syn_value::FXSynValueArg;
pub use crate::traits::*;
pub use crate::value::FXValueArg;
pub use crate::with_origin::FXOrig;

pub use fieldx_derive_support::fxhelper;
use quote::quote;
use quote::ToTokens;
use syn::ext::IdentExt;
use value::FXEmpty;

/// Concurrency mode
///
/// In particular, specifies what default types and containers are used. For example, refernce counted objects would use
/// [`std::sync::Arc`] for `sync` and `async`, but [`std::rc::Rc`] for `plain`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum FXSyncModeVariant {
    /// sync mode
    Sync,
    /// async mode
    Async,
    /// plain, i.e. non-concurrent
    #[default]
    Plain,
}

#[derive(Debug, Clone)]
pub struct FXSyncMode {
    variant: FXSyncModeVariant,
    orig:    Option<syn::Ident>,
}

impl syn::parse::Parse for FXSyncMode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse_any(input)?;
        let variant = if ident == "sync" {
            FXSyncModeVariant::Sync
        }
        else if ident == "async" {
            FXSyncModeVariant::Async
        }
        else if ident == "plain" {
            FXSyncModeVariant::Plain
        }
        else {
            Err(syn::Error::new_spanned(
                ident.to_token_stream(),
                "expected 'sync', 'async' or 'plain'",
            ))?
        };
        Ok(FXSyncMode {
            variant,
            orig: Some(ident),
        })
    }
}

impl FXSyncMode {
    pub fn is_sync(&self) -> bool {
        self.variant == FXSyncModeVariant::Sync
    }

    pub fn is_async(&self) -> bool {
        self.variant == FXSyncModeVariant::Async
    }

    pub fn is_plain(&self) -> bool {
        self.variant == FXSyncModeVariant::Plain
    }

    // Only to make it usable with validate_exclusives macro
    pub fn is_true(&self) -> FXProp<bool> {
        FXProp::new(true, None)
    }

    pub fn variant(&self) -> FXSyncModeVariant {
        self.variant
    }
}

impl ToTokens for FXSyncMode {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if let Some(ref orig) = self.orig {
            tokens.extend(orig.to_token_stream());
        }
        else {
            tokens.extend(match self.variant {
                FXSyncModeVariant::Sync => quote![sync],
                FXSyncModeVariant::Async => quote![async],
                FXSyncModeVariant::Plain => quote![plain],
            });
        }
    }
}

/// Standard helper
pub type FXHelper<const BOOL_ONLY: bool = false> = FXNestingAttr<FXBaseHelper<BOOL_ONLY>>;
/// Standard literal value
pub type FXValue<T, const BOOL_ONLY: bool = false> = FXNestingAttr<FXValueArg<T, BOOL_ONLY>>;
/// Standard type implementing [`syn::parse::Parse`]
pub type FXSynValue<T, const AS_KEYWORD: bool = false> = FXNestingAttr<FXSynValueArg<T, AS_KEYWORD>, false>;
/// Standard tuple for types implementing [`syn::parse::Parse`]
pub type FXSynTuple<T> = FXNestingAttr<FXSynTupleArg<T>, false>;
/// String literal
pub type FXString = FXNestingAttr<FXValueArg<String>>;
/// Boolean literal
pub type FXBool = FXNestingAttr<FXValueArg<FXEmpty, true>>;
/// Accessor helper
pub type FXAccessor<const BOOL_ONLY: bool = false> = FXNestingAttr<FXAccessorHelper<BOOL_ONLY>>;
/// Setter helper
pub type FXSetter<const BOOL_ONLY: bool = false> = FXNestingAttr<FXSetterHelper<BOOL_ONLY>>;
/// Builder helper
pub type FXBuilder<const STRUCT: bool = false> = FXNestingAttr<FXBuilderHelper<STRUCT>>;
/// `serde` argument
pub type FXSerde<const STRUCT: bool = false> = FXNestingAttr<FXSerdeHelper<STRUCT>>;
/// `doc` argument
pub type FXDoc = FXNestingAttr<FXDocArg>;

pub type FXAttributes = FXSynValue<FXPunctuated<FXAttribute, syn::Token![,]>>;

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn fx_attributes() {
        let attrs: FXPunctuated<FXAttribute, syn::Token![,]> =
            parse_quote! {allow(unused), derive(Debug), serde(rename_all="lowercase")};

        assert_eq!(attrs.iter().count(), 3);

        let al = attrs
            .iter()
            .map(|a| a.path().get_ident().to_token_stream().to_string())
            .collect::<Vec<_>>();
        assert_eq!(al, vec!["allow", "derive", "serde"]);
    }
}
