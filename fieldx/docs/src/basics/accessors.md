# Accessors

In FieldX, there are two ways to get an {{i:accessor}} method for a field:

- Explicitly, by using the `get` argument.
- Implicitly, when the use of another argument makes no sense without an accessor.

There are also two kinds of accessors: immutable{{hi:immutable accessor}} and mutable{{hi:mutable accessor}}. The latter is never generated implicitly and is only available with the use of the `get_mut` argument.

By default, an accessor returns a reference to the field value[^unless_other_args]:

```rust,ignore
{{#include ../../../examples/book_accessor.rs:ref_decl}}

{{#include ../../../examples/book_accessor.rs:ref_usage}}
```

So far, so good! But, wait, **_year_**? This is disgusting!

```rust,ignore
{{#include ../../../examples/book_accessor.rs:copy_decl}}

{{#include ../../../examples/book_accessor.rs:copy_usage}}
```

Now, this is much better! The `get(copy)` can be used with types that implement the `Copy` trait, such as `usize`, `u32`, etc., to get the value itself rather than a reference to it.

Along the lines, we use this sample to showcase two more things:

- The `get(clone)` can be used with types that implement the `Clone` trait, such as `String`, to get a cloned value.
- It is possible to override a struct-level default argument for a specific field, like we did with `get(copy)` for the `year` field and `get(clone)` for the `author` field.

And finally, let's have a quick look at the mutable accessors:

```rust,ignore
{{#include ../../../examples/book_accessor.rs:mut_decl}}

{{#include ../../../examples/book_accessor.rs:mut_usage}}
```

```admonish
As helper attributes, `get` and `get_mut` can take a literal value sub-argument that would give a name to the accessor method when used at the field level, or define a custom method name prefix for default accessor names when used at the struct level.
```

```rust,ignore
{{#include ../../../examples/book_accessor.rs:rename_decl}}

{{#include ../../../examples/book_accessor.rs:rename_usage}}
```
