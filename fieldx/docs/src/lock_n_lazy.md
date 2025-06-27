# Lock'n'Lazy

<!-- TODO! Mention the `writer` argument in the context of being able to write into a lazy field without initiating its builder method. -->

The general rule FieldX tries to follow is the KISS principle. It avoids introducing unnecessary entities without a good reason to do otherwise. In the context of this chapter, let's mention that the {{i:lazy field initialization}} is based upon various flavors of the `OnceCell` type, while {{i:locks}} are implemented using the `RwLock<T>` types (unless the `clonable-lock` feature flag, discussed in the [previous chapter](./more_on_locks.md)). But the combination of the two for a field leads to a couple of problems that do not have a straightforward solution.

Say, do we wrap the `OnceCell` in a lock or the other way around? Sync/async versions of `OnceCell` itself are lock-protected internally anyway, how do we avoid double locking?

Instead of trying to fuse together two things that are not meant to be fused, FieldX introduces its own solution: {{i:`FXProxy`}} types[^type_plurality] that implement both laziness and locking "all-in-one." And some more.

[^type_plurality]: The plural form is used here to indicate that there are multiple `FXProxy` types, each of which is specialized for a specific mode of operation.

In most of the cases a user wouldn't need to worry about what container type a field is using as long as they're using method-based access to it. However, it worth paying attention to the type documentation if you plan to use the field directly – not only to understand possible nuances of the type implementation, but also find out about some additional functionality it provides.

At this point we need to focus on an aspect that is not immediately obvious. Let's recall about the [Readers And Writers](./basics/locks.md#readers-and-writers) section in the Locks chapter. The time has come to explain the mystery around the `writer` argument and what makes it different from the mutable accessor.

As nearly always, a mistification revealed becomes ridiculously simple: no matter, mutable or not, an accessor remains an accessor and as such its job is to first initialize a lazy field first if it is not initialized yet. Therefore, if you only want to put a value into the field you'd anyway pay the cost of calling its builder method – which might be expensive at times!

Contrary, the writer method gives you a direct access into the container without the need to initialize it first – but without the ability to read a value from it! This functionality is backed by the `FXProxy` types[^type_plurality] which `write` method return {{i:`FXWriter`}} guards that have only two methods: `clear` and `store`:

```rust,ignore
{{#include ../../examples/book_lazy_lock.rs:simple_decl}}

{{#include ../../examples/book_lazy_lock.rs:simple_test}}
```
