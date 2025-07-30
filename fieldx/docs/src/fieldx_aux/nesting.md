# Nesting

The term {{i:nesting}} comes from the [`darling::ast::NestedMeta`] and [`syn::meta::ParseNestedMeta`]. In FieldX' context specifically nesting means "nesting of arguments and sub-arguments" – see [Terminology](../basics/terminology.md#arguments).

The two cornerstones of nesting are the [`FXNestingAttr`][fieldx_aux::FXNestingAttr] {{hi:FXNestingAttr}} type and [`FromNestAttr`][fieldx_aux::FromNestAttr] {{hi:FromNestAttr}} trait.

The `FXNestingAttr` type is the one that takes the burden of parsing a function-like syntax. But before we get into details let's consider what exactly it parses.

## Literal And Non-literal Arguments
