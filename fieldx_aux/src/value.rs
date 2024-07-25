use super::{FXFrom, FXTriggerHelper, FromNestAttr};
use darling::{util::Flag, FromMeta};
use syn::Lit;

#[derive(Debug, FromMeta, Clone)]
pub struct FXValueArg<T, const BOOL_ONLY: bool = false> {
    off:   Flag,
    #[darling(skip)]
    value: Option<T>,
}

impl<T, const BOOL_ONLY: bool> FXValueArg<T, BOOL_ONLY> {
    fn validate_literals(literals: &Vec<Lit>) -> darling::Result<()> {
        if literals.len() > 0 && BOOL_ONLY {
            Err(darling::Error::custom("No literal arguments are allowed here").with_span(&literals[0].span()))
        }
        else if literals.len() > 1 {
            Err(darling::Error::custom("Only one literal argument is allowed here").with_span(&literals[0].span()))
        }
        else {
            Ok(())
        }
    }

    fn as_keyword(path: &syn::Path) -> darling::Result<Self> {
        if BOOL_ONLY {
            Ok(Self {
                off:   Flag::default(),
                value: None,
            })
        }
        else {
            Err(darling::Error::custom(format!("A literal {} argument is required", stringify!(T))).with_span(path))
        }
    }

    pub fn value(&self) -> Option<&T> {
        if self.is_true() {
            self.value.as_ref()
        }
        else {
            None
        }
    }
}

impl<T, const BOOL_ONLY: bool> FXTriggerHelper for FXValueArg<T, BOOL_ONLY> {
    fn is_true(&self) -> bool {
        !self.off.is_present()
    }
}

impl<T, const BOOL_ONLY: bool> From<T> for FXValueArg<T, BOOL_ONLY> {
    fn from(value: T) -> Self {
        Self {
            off:   Flag::default(),
            value: Some(value),
        }
    }
}

macro_rules! from_nest_attr_num {
    ($from:path => $ty:ty) => {
        impl crate::FromNestAttr for FXValueArg<$ty, false> {
            fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
                Self::validate_literals(literals)?;
                if let $from(ref lit) = literals[0] {
                    self.value = Some(lit.base10_parse()?);
                }
                else {
                    return Err(darling::Error::unexpected_lit_type(&literals[0]));
                }
                Ok(self)
            }

            fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
                Self::as_keyword(path)
            }
        }
    };
}

impl FromNestAttr for FXValueArg<(), true> {
    fn set_literals(self, literals: &Vec<Lit>) -> darling::Result<Self> {
        Self::validate_literals(literals)?;
        Ok(self)
    }

    fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
        Self::as_keyword(path)
    }
}

impl<T, const BOOL_ONLY: bool> FXFrom<T> for FXValueArg<T, BOOL_ONLY> {
    fn fx_from(value: T) -> Self {
        Self {
            value: Some(value),
            off:   Flag::default(),
        }
    }
}

impl<T, const BOOL_ONLY: bool> FXFrom<Option<T>> for FXValueArg<T, BOOL_ONLY> {
    fn fx_from(value: Option<T>) -> Self {
        Self {
            value,
            off: Flag::default(),
        }
    }
}

macro_rules! from_nest_attr_val {
    ($from:path => $ty:ty) => {
        impl crate::FromNestAttr for FXValueArg<$ty, false> {
            fn set_literals(mut self, literals: &Vec<Lit>) -> darling::Result<Self> {
                Self::validate_literals(literals)?;
                if let $from(ref lit) = literals[0] {
                    self.value = Some(lit.value());
                }
                else {
                    return Err(darling::Error::unexpected_lit_type(&literals[0]));
                }
                Ok(self)
            }

            fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
                Self::as_keyword(path)
            }
        }
    };
}

from_nest_attr_num!(Lit::Int => i8);
from_nest_attr_num!(Lit::Int => i16);
from_nest_attr_num!(Lit::Int => i32);
from_nest_attr_num!(Lit::Int => i64);
from_nest_attr_num!(Lit::Int => u8);
from_nest_attr_num!(Lit::Int => u16);
from_nest_attr_num!(Lit::Int => u32);
from_nest_attr_num!(Lit::Int => u64);
from_nest_attr_num!(Lit::Float => f32);
from_nest_attr_num!(Lit::Float => f64);
from_nest_attr_val!(Lit::Str => String);
from_nest_attr_val!(Lit::ByteStr => Vec<u8>);
from_nest_attr_val!(Lit::Char => char);
from_nest_attr_val!(Lit::Bool => bool);

// #[cfg(test)]
// mod test {
//     use darling::ast::NestedMeta;
//     use syn::parse_quote;

//     use super::*;

//     #[derive(Debug, Default, FromMeta)]
//     struct Foo {
//         some: Option<FXValueArg<String>>,
//     }

//     #[test]
//     fn test_from_nest_attr_num() {
//         let meta: NestedMeta = parse_quote![some("aa")];
//         let NestedMeta::Meta(meta) = meta else { panic!("expected meta") };
//         eprintln!("META: {:#?}", meta);
//         let foo = Foo::from_meta(&meta).unwrap();
//         // // let nest = <FXNestingAttr<FXValueArg<String>>>::from_list(&[meta]).expect("FXValueArg parsed");
//         // eprintln!("nest: {:?}", foo);
//         // // assert_eq!(nest.value(), &Some("aa".to_string()));
//     }
// }
