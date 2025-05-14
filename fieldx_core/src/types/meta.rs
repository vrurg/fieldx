use proc_macro2::TokenStream;
use quote::ToTokens;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FXValueFlag {
    #[default]
    None             = 0,
    /// Value is a standard default value.
    StdDefault       = 1,
    /// Value is an user-introduced default value.
    UserDefault      = 2,
    /// Value is wrapped in a container type.
    ContainerWrapped = 4,
    /// Value is wrapped in a reference countered type.
    RefCounted       = 8,
}

/// This is a container for values that can be passed through many transformations while retaining their value context,
/// whether original or introduced by a transformation.
#[derive(Debug, Clone)]
pub struct FXValueMeta<T> {
    pub value:      T,
    pub flags:      u8,
    pub attributes: Vec<TokenStream>,
}

impl<T> FXValueMeta<T> {
    pub fn new(value: T, flag: FXValueFlag) -> Self {
        let flags = flag as u8;
        Self {
            value,
            flags,
            attributes: Vec::new(),
        }
    }

    #[inline]
    pub fn mark_as(mut self, flag: FXValueFlag) -> Self {
        self.flags |= flag as u8;
        self
    }

    #[inline]
    pub fn has_flag(&self, flag: FXValueFlag) -> bool {
        self.flags & (flag as u8) != 0
    }

    #[inline]
    pub fn add_attribute<TT: ToTokens>(mut self, attribute: TT) -> Self {
        self.attributes.push(attribute.to_token_stream());
        self
    }

    #[inline]
    pub fn replace(self, value: T) -> Self {
        Self {
            value,
            flags: self.flags,
            attributes: self.attributes,
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: IntoIterator> IntoIterator for FXValueMeta<T> {
    type IntoIter = T::IntoIter;
    type Item = T::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

impl<T> From<T> for FXValueMeta<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self {
            value,
            flags: FXValueFlag::None as u8,
            attributes: Vec::new(),
        }
    }
}

impl<T: ToTokens> ToTokens for FXValueMeta<T> {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

pub type FXToksMeta = FXValueMeta<TokenStream>;
