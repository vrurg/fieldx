#![doc(html_root_url = "https://docs.rs/fieldx_aux/")]
//! # fieldx_aux
//!
//! Helper module for the [`fieldx`] crate and for any 3rd party crates, extending its functionality.
//!
//! `fieldx` itself is heavily based on [`darling`] crate which simplifies development of proc-macros quite a lot. But
//! it also imposes some constraints on attribute arguments syntax. This crate aims at overcoming these limitations and
//! providing support for some kinds of attributes required to implement `fieldx`.
//!
//! Here is a little break down of what is provided:
//!
//! - support for nested arguments, i.e. those that look like `arg1("value", trigger, subarg(...))`
//! - support for some syntax elements that are not on the `darling` crate menu: `some_type(crate::types::Foo)`,
//!   `error(crate::error::Error, crate::error::Error::SomeProblem("with details"))`[^tuple]
//! - a set of types implementing standard `fieldx` arguments like helpers, or literal values, etc.
//!
//! [^tuple]: Here, the first argument of `error()` — `Error` — is an enum; `SomeProblem` is a variant.
//!
//! # Usage
//!
//! Let's say we're implementing a field-level attribute `foo` using [`darling::FromField`] trait. And we want it to
//! take these arguments:
//!
//! - `trigger` which would let turn some functionality on or off
//! - `action` to specify a method with special meaning
//! - `comment` with some text
//! - `vis` to specify if field-related code must be public and if yes then what kind of `pub` we need
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
//! For this we'd need the following declaration somewhere in our proc-macro implementation:
//!
//! ```ignore
//! #derive(FromField)
//! #[darling(attributes(foo))]
//! struct FooField {
//!     // ... skipping some darling-default fields ...
//!
//!     trigger: Option<FXBool>,
//!     action: Option<FXHelper>,
//!     comment: Option<FXString>,
//!     vis: Option<FXSynValue<syn::Visibility>>,
//! }
//! ```
//!
//! That's all, this crate will take the burden of implementing the arguments from you!
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
use syn::ext::IdentExt;
use value::FXEmpty;

/// Concurrency mode
///
/// In particular, specifies what default types and containers are used. For example, refernce counted objects would use
/// [`std::sync::Arc`] for `sync` and `async`, but [`std::rc::Rc`] for `plain`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum FXSyncMode {
    /// sync mode
    Sync,
    /// async mode
    Async,
    /// plain, i.e. non-concurrent
    #[default]
    Plain,
}

impl syn::parse::Parse for FXSyncMode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse_any(input)?;
        Ok(if ident == "sync" {
            Self::Sync
        }
        else if ident == "async" {
            Self::Async
        }
        else if ident == "plain" {
            Self::Plain
        }
        else {
            Err(syn::Error::new_spanned(ident, "expected 'sync', 'async' or 'plain'"))?
        })
    }
}

impl FXSyncMode {
    pub fn is_sync(&self) -> bool {
        self == &Self::Sync
    }

    pub fn is_async(&self) -> bool {
        self == &Self::Async
    }

    pub fn is_plain(&self) -> bool {
        self == &Self::Plain
    }

    // Only to make it usable with validate_exclusives macro
    pub fn is_true(&self) -> FXProp<bool> {
        FXProp::new(true, None)
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
pub type FXSerde = FXNestingAttr<FXSerdeHelper>;
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
