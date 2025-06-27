# Visibility

It is possible to control the {{i:visibility}} of FieldX-generated entities. This can be done by using the `vis(...)` argument (or sub-argument) with corresponding visibility levels using the [`pub` declaration](https://doc.rust-lang.org/reference/visibility-and-privacy.html). Private visibility is achieved by using no sub-argument with `vis`: `vis()`; there is also an alias `private` for it.

By default, the {{i:visibility level}} is inherited from the field or the struct declaration. More precisely, there is a priority order of visibility levels that FieldX follows:

1. The immediate declaration for an entity: `get(vis(pub))`.
2. Field-level declaration: `#[fieldx(vis(pub), get)]`.
3. Struct-level default for the entity: `#[fxstruct(get(vis(pub)))]`.
4. Struct-level default: `#[fxstruct(vis(pub))]`.
5. Field **explicit** declaration: `pub foo: usize`.
6. Struct declaration: `pub struct Foo { ... }`.

```admonish warning
Don't ignore the "explicit" word at the field level! It means that if the field is private, FieldX skips the step and goes directly to the struct declaration. The reason for this is that the struct is considered part of some kind of API of a module, and as such, it is better for the methods it provides to be exposed at the same level because they're part of the same API.

Think of it in terms of a field accessor where the field itself is recommended to be private, while the accessor is recommended to be accessible anywhere the struct is accessible.
```

```rust,ignore
{{#include ../../../examples/book_visibility.rs:visibility_decl}}
```

The levels in the snippet are somewhat arbitrary for the sake of demonstrating the feature.
