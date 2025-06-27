# Locks {{hi:locks}}

Support for sync/async modes of operation is incomplete without providing a way to lock-protect data. FieldX can do this for you at the field level by using the {{i:`lock`}} argument.

```admonish
Using `inner_mut` with explicit `sync` or `mode(sync)` has the same effect as using the `lock` argument.
```

```admonish
The async mode still requires the `async` argument to be used.
```

For greater flexibility, FieldX utilizes read-write locks provided by the [`parking_lot`](https://docs.rs/parking_lot/latest/parking_lot/) crate for the sync mode, and either [`tokio`](https://docs.rs/tokio/latest/tokio/) or [`async-lock`](https://docs.rs/async-lock/latest/async_lock/) for the async mode, depending on which [{{i:feature flag}}](../feature_flags.md) is enabled. This design decision aligns well with the immutable and mutable accessor patterns.

Making a field lockable affects the return values of its accessors. Without focusing on specific `sync` or `async` modes, the immutable accessor of such a field returns an `RwLockReadGuard`, while the mutable accessor returns an `RwLockWriteGuard`:

```rust,ignore
{{#include ../../../examples/book_lock.rs:simple_decl}}

{{#include ../../../examples/book_lock.rs:simple_test}}
```

The use of the struct and its locked fields is straightforward in the example above. What is really worth mentioning are the following two points:

1. Implicit sync mode when the `lock` argument is used. This means there is no need to specify `sync` or `mode(sync)` per field if it is already locked.
2. The `lock` argument itself can be implicit when `reader` or `writer` arguments are used. Apparently, the `sync` mode is also implied in this case.

## Readers And Writers

The {{i:`reader`}} and {{i:`writer`}} arguments are additional helpers that introduce methods always returning an `RwLockReadGuard` and `RwLockWriteGuard`, respectively. The methods are named after the field they are applied to, with the `read_` and `write_` prefixes added.

At this point, an immediate question arises: what makes them different from the immutable and mutable accessors? For the `reader`, the answer is the word "always" in the previous paragraph. Consider the `copy` subargument of the `available` field's `get` – with it, the locking happens inside the accessor, and we only get a `u32` value.

For the `writer`, the situation is different and slightly more intricate. It will be discussed in the [Lock'n'Lazy](../lock_n_lazy.md) chapter.
