# Mutability

You probably have noticed that both the mutable accessor and setter require the object itself to be mutable. While this is a common requirement in Rust, it is not always possible to fulfill. This is where [{{i:interior mutability}}](https://doc.rust-lang.org/reference/interior-mutability.html) comes into play. FieldX provides support for this pattern via the `inner_mut` argument, which, combined with the `get_mut` and `set` arguments, allows you to mutate a field even when the object itself is not mutable.

```rust,ignore
{{#include ../../../examples/book_inner_mut.rs:imut_decl}}

{{#include ../../../examples/book_inner_mut.rs:imut_usage}}
```

```admonish
In the above example, while nothing has changed for the `available` field accessor, which carries the `copy` sub-argument, the accessor for the `location` field now requires dereferencing. This is because both field types are now wrapped in a [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html) container which, when borrowed, returns a `Ref` type. The `copy` sub-argument simplifies this since returning a value is straightforward. However, the `location` field accessor still returns a reference, just a different kind of it.
```

```admonish
In sync and async modes of operation use of the `inner_mut` argument is equivalent to using [`lock`](./modes_of_operation.md).
```
