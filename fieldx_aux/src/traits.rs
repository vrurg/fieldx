use crate::{FXAttributes, FXPubMode};

/// Trait for arguments with trigger behavior. For example, `fieldx` `get` which can be disabled by `off` subargument.
pub trait FXTriggerHelper {
    /// Trigger value
    fn is_true(&self) -> bool;
}

/// Where it is not possible to use the standard `From`/`Into` traits due to conflicting implementations this crate is
/// using `FXFrom`/`FXInto` instead.
pub trait FXFrom<T> {
    fn fx_from(value: T) -> Self;
}

/// The counterpart of `FXFrom`.
pub trait FXInto<T> {
    fn fx_into(self) -> T;
}

/// Implements `FXTriggerHelper`-like functionality for types that are optional.
pub trait FXBoolHelper {
    fn is_true(&self) -> bool;
    fn is_true_opt(&self) -> Option<bool>;
    fn not_true(&self) -> bool {
        !self.is_true()
    }
}

/// Base functionality of helper types.
pub trait FXHelperTrait: FXTriggerHelper {
    /// Helper method name.
    fn name(&self) -> Option<&str>;
    /// Public mode for generated helper. Must be private if `None`
    fn public_mode(&self) -> Option<FXPubMode>;
    /// For helper methods that are backed by additional types these are attributes to be applied to the types.
    fn attributes(&self) -> Option<&FXAttributes>;
    /// Additional attributes to apply to generated helper.
    fn attributes_fn(&self) -> Option<&FXAttributes>;
}

impl<T, U> FXInto<U> for T
where
    U: FXFrom<T>,
{
    #[inline]
    fn fx_into(self) -> U {
        U::fx_from(self)
    }
}

impl<H: FXTriggerHelper> FXBoolHelper for Option<H> {
    #[inline]
    fn is_true(&self) -> bool {
        self.as_ref().map_or(false, |h| h.is_true())
    }

    #[inline]
    fn is_true_opt(&self) -> Option<bool> {
        self.as_ref().map(|h| h.is_true())
    }
}
