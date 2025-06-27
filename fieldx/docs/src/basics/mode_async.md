# Async

The _{{i:async}}_ mode of operation is identical to the sync mode in many respects. They even share the same code generator behind the scenes. Yet, there are details that warrant additional attention.

The primary use of _async_ is with lazily initialized fields and locks. Let's have a look at why this is the case.

## Locks

An early FieldX design didn't provide async support at all. While for lazies it was a nuisance, for locks it was considered acceptable only until the author was hard bitten by a lock that actually blocked a tokio task thread, effectively freezing all tasks running on the thread—whereas one of the tasks was supposed to actually release the lock. You got it!

That's all there is to it. With the use of async locks, the above situation wouldn't happen (in that particular case); or, at least, deadlocks would have a less severe impact on the system.

## Lazies

Lazy field initialization is another driving force behind the introduction of the async mode. Consider initializing a network resource in an async context, for example. Either this brings us back to the boilerplate of manually implementing laziness, or to the boilerplate of creating a new runtime only to use the async code in a builder method! A choice without a choice...

Let's have a look at what we can do rather than reflecting on what we wouldn't:

```rust,ignore
{{#include ../../../examples/book_async.rs:simple_decl}}

{{#include ../../../examples/book_async.rs:simple_usage}}
```

As usual, nothing too complicated here. The accessor methods become `async`, which is expected because they are the ones that call the builder method.

## Backend Choice

FieldX provides two options for the async backend: `tokio` and `async-lock`. These backends are utilized for their implementations of `RwLock` and `OnceCell`. The selection is controlled via the {{i:`async-tokio`}} or {{i:`async-lock`}} [feature flags](../feature_flags.md), with `async-tokio` being the default.
