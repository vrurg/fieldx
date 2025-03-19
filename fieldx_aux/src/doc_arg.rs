use crate::{FXProp, FXSetState, FromNestAttr};
use darling::FromMeta;

/// Minimal helper declaration. For example, `fieldx` uses it for helpers like `reader` or `writer`.
///
/// The `BOOL_ONLY` parameter disables the literal subargument that specifies custom helper name. For example, with
/// the following declaraion, argument `foo("my_name")` results in an error:
///
/// ```ignore
///     foo: FXNestingAttr<FXBaseHelper<true>>,
/// ```
#[derive(Default, Debug, Clone, FromMeta)]
pub struct FXDocArg {
    #[darling(skip)]
    lines: Vec<syn::LitStr>,
}

impl FXDocArg {
    /// Shortcut to the `lines` parameter.
    pub fn lines(&self) -> FXProp<Vec<syn::LitStr>> {
        FXProp::new(self.lines.clone(), None)
    }
}

impl From<&FXDocArg> for Option<FXProp<Vec<syn::LitStr>>> {
    fn from(arg: &FXDocArg) -> Self {
        Some(FXProp::new(arg.lines.clone(), None))
    }
}

impl FromNestAttr<true> for FXDocArg {
    fn set_literals(mut self, literals: &Vec<syn::Lit>) -> darling::Result<Self> {
        self.lines = literals
            .iter()
            .map(|lit| {
                if let syn::Lit::Str(lit) = lit {
                    Ok(lit.clone())
                }
                else {
                    Err(darling::Error::custom("Only string literals are allowed here").with_span(&lit.span()))
                }
            })
            .collect::<darling::Result<Vec<_>>>()?;
        Ok(self)
    }
}

impl FXSetState for FXDocArg {
    // Consider itself as always set to allow for empty doc.
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(true, None)
    }
}
