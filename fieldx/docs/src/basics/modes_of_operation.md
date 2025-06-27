# Modes Of Operation

There is no concise way to explain how a [mode of operation](./terminology.md#modes-of-operation) works for every specific case in FieldX. Up to some limited extent, one could say that this entire book is dedicated to exploring this topic. However, here are a few points to help you get started.

## Declaration

The arguments that set one of the modes are either named after the modes themselves: `plain`, `sync`, or `r#async`, where the `r#` is required to allow use of the `async` keyword as an argument name; or these keywords can be used as sub-arguments of the {{i:`mode`}} argument: `mode(async)`. In the latter case, the `r#` prefix before `async` is not required.

Both options are entirely equivalent; choose the one that is more readable for your use case.

## Plain

The _plain_ mode is the default mode of operation for FieldX. It does not provide any guarantees regarding thread safety or concurrency.

## Sync

Suppose you have a struct `Foo` with some fields that are not [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html) or [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html). If the struct is annotated with the `#[fxstruct]` attribute, we say it operates in _plain_ mode. At some point, you may realize that you need to use `Foo` in a concurrent context. Often, it may be enough to add the `sync` argument to the `#[fxstruct]` attribute, like this:

```rust,ignore
#[fxstruct(sync)]
struct Foo {
    // ...
}
```

Now, `Foo` operates in _sync_ mode and implements the `Sync+Send` traits. The specific meaning of adding the `sync` argument may vary for each individual field of `Foo`, depending on their declarations. Let's consider one example to illustrate this:

```rust,ignore
#[fxstruct(get)]
struct Foo {
    #[fieldx(inner_mut, set)]
    value: String,
}
```

Here, the `value` field is a plain one. The `inner_mut` attribute enables the implementation of the [inner mutability pattern](https://docs.rs/fieldx/latest/fieldx/attr.fxstruct.html#inner_mut-1) for the field. Technically, this means the field type is now wrapped in [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html), which is `Send` but not `Sync`. So, let's add the argument:

```rust,ignore
#[fxstruct(sync, get)]
struct Foo {
    #[fieldx(inner_mut, set)]
    value: String,
}
```

Now FieldX will use `RwLock` instead of `RefCell`[^inner_mut_vs_lock]. In an ideal world, not only would we not need to change any of the `foo.set(new_value)` calls, but as long as all our accessor calls are dereferenced, their usage will remain the same. Moreover, if for some reason we always need a clone of the `value` field, then with the following declaration, all uses of the `value` accessor will remain the same without any caveats:

```rust,ignore
#[fxstruct(sync)]
struct Foo {
    #[fieldx(inner_mut, get(clone), set)]
    value: String,
}
```

[^inner_mut_vs_lock]: The `inner_mut` and `lock` arguments are aliases for the same functionality in `sync` and `async` modes. The `lock` argument exists for readability and convenience, as marking a field as _locked_ automatically implies that it is `sync`.

## Async

The _async_ mode is covered in a [later chapter](./mode_async.md).

## Struct or Field?

Since FieldX is primarily a field-oriented crate, it allows manipulation of operation modes for individual fields. For example, with the `Foo` struct from the examples above, you could do the following:

```rust,ignore
#[fxstruct]
struct Foo {
    #[fieldx(inner_mut, sync, set)]
    value: String,
    // Or `r#async` because keywords can't be used as first-level argument names.
    #[fieldx(lock, mode(async))]
    async_value: String,
}
```
