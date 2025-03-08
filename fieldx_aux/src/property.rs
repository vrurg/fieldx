use std::ops::Deref;

use darling::util::Flag;
use proc_macro2::Span;

use crate::{FXNestingAttr, FXTriggerHelper, FXValueArg};

#[derive(Debug)]
pub struct FXProp<T> {
    value: T,
    span:  Option<Span>,
}

impl<T> FXProp<T> {
    pub fn new(value: T, span: Option<Span>) -> Self {
        Self { value, span }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn orig_span(&self) -> Option<Span> {
        self.span
    }

    pub fn final_span(&self) -> Span {
        self.span.unwrap_or_else(Span::call_site)
    }

    // Set span if it is not already set. Doesn't overwrite existing span.
    pub fn respan(mut self, span: Option<Span>) -> Self {
        if span.is_some() && self.span.is_none() {
            self.span = span;
        }
        self
    }
}

impl FXProp<bool> {
    pub fn true_or_none(&self) -> Option<&Self> {
        if self.value {
            Some(self)
        }
        else {
            None
        }
    }
}

impl<T> FXProp<T>
where
    T: FXTriggerHelper,
{
    pub fn or<'a>(&'a self, other: &'a FXProp<T>) -> &'a Self {
        if *self.is_true() {
            self
        }
        else {
            other
        }
    }
}

impl<T> Default for FXProp<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            value: Default::default(),
            span:  None,
        }
    }
}

impl<T> Deref for FXProp<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Clone for FXProp<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            span:  self.span,
        }
    }
}

impl<T> Copy for FXProp<T> where T: Copy {}

impl<T> From<FXProp<T>> for bool
where
    T: FXTriggerHelper,
{
    fn from(value: FXProp<T>) -> Self {
        *value.is_true()
    }
}

impl From<FXProp<bool>> for bool {
    fn from(value: FXProp<bool>) -> Self {
        value.value
    }
}

impl<T> From<&FXProp<T>> for bool
where
    T: FXTriggerHelper,
{
    fn from(value: &FXProp<T>) -> Self {
        *value.is_true()
    }
}

impl From<&FXProp<bool>> for bool {
    fn from(value: &FXProp<bool>) -> Self {
        value.value
    }
}

impl From<Flag> for FXProp<bool> {
    fn from(value: Flag) -> Self {
        Self::new(value.is_present(), Some(value.span()))
    }
}

impl From<&Flag> for FXProp<bool> {
    fn from(value: &Flag) -> Self {
        Self::new(value.is_present(), Some(value.span()))
    }
}

impl From<bool> for FXProp<bool> {
    fn from(value: bool) -> Self {
        Self::new(value, None)
    }
}

// impl<T> From<T> for FXProp<bool>
// where
//     T: FXTriggerHelper,
// {
//     fn from(value: T) -> Self {
//         value.is_true()
//     }
// }

impl<T> From<&T> for FXProp<bool>
where
    T: FXTriggerHelper,
{
    fn from(value: &T) -> Self {
        value.is_true()
    }
}

impl<T> From<FXProp<T>> for FXProp<bool>
where
    T: FXTriggerHelper,
{
    fn from(value: FXProp<T>) -> Self {
        value.is_true()
    }
}

impl<T> From<&FXProp<T>> for FXProp<bool>
where
    T: FXTriggerHelper,
{
    fn from(value: &FXProp<T>) -> Self {
        value.is_true()
    }
}

pub trait FXPropBool {
    type Or;
    type Not;

    fn or(self, other: Self) -> Self::Or;
    fn not(self) -> Self::Not;
}

impl<'a> FXPropBool for &'a FXProp<bool> {
    type Not = FXProp<bool>;
    type Or = &'a FXProp<bool>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        if self.value {
            self
        }
        else {
            other
        }
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        FXProp::new(!self.value, self.span)
    }
}

impl FXPropBool for FXProp<bool> {
    type Not = FXProp<bool>;
    type Or = FXProp<bool>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        if self.value {
            self
        }
        else {
            other
        }
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        FXProp::new(!self.value, self.span)
    }
}

impl<'a> FXPropBool for Option<&'a FXProp<bool>> {
    type Not = Option<FXProp<bool>>;
    type Or = Option<&'a FXProp<bool>>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        self.and_then(|s| s.true_or_none())
            .or_else(|| other.and_then(|o| o.true_or_none()))
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        self.map(|f| FXProp::new(!f.value, f.span))
    }
}

impl FXPropBool for Option<FXProp<bool>> {
    type Not = Option<FXProp<bool>>;
    type Or = Option<FXProp<bool>>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        self.and_then(|s| s.true_or_none().copied())
            .or_else(|| other.and_then(|o| o.true_or_none().copied()))
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        self.map(|f| FXProp::new(!f.value, f.span))
    }
}

impl<'a> FXPropBool for &'a Option<FXProp<bool>> {
    type Not = Option<FXProp<bool>>;
    type Or = Option<&'a FXProp<bool>>;

    #[inline(always)]
    fn or(self, other: Self) -> Self::Or {
        self.as_ref()
            .and_then(|s| s.true_or_none())
            .or_else(|| other.as_ref().and_then(|o| o.true_or_none()))
    }

    #[inline(always)]
    fn not(self) -> Self::Not {
        self.map(|f| FXProp::new(!f.value, f.span))
    }
}
