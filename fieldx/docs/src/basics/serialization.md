# Serialization

```admonish warning
FieldX uses the [serde](https://serde.rs/) crate to support {{i:serialization}}. Understanding this chapter may sometimes require some knowledge of the crate.
```

Serialization in FieldX is enabled with the `serde` {{i:feature flag}} and the {{i:`serde` argument}}. However, these are not the only prerequisites, and to understand why, some explanations are needed.

## The Complications and the Solutions

At this point, it's time to rewind to the [Basics](../basics.md) chapter and recall that under the hood, FieldX transforms the struct by wrapping its fields into container types if necessary. Direct serialization of some of these containers is not possible or may result in undesired outcomes. Generally speaking, using FieldX could have made serialization impossible unless the `serde` crate lends us a helpful hand!

The solution is to use the [`from`](https://serde.rs/container-attrs.html#from) and [`into`](https://serde.rs/container-attrs.html#into) attributes of `serde`, implement a copy of the user struct with containers stripped away, and use it for the actual serialization. FieldX calls this a {{i:shadow struct}}.

To support this functionality, the user struct must implement the [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html) trait, which is a prerequisite for applying the `serde` argument. Since cloning a struct can be a non-trivial task, it is left to the user. In most cases, deriving it should suffice.

## How To in a Nutshell

As is common with FieldX, the first step in serialization is as simple as adding the `serde` argument to the struct declaration:

```rust,ignore
{{#include ../../../examples/book_serde.rs:ser_decl}}
```

And we're ready to use it:

```rust,ignore
{{#include ../../../examples/book_serde.rs:ser_usage}}
```

~~~admonish info title="Output"
```json
<!-- cmdrun cargo run --features serde --example book_serde -->
```
~~~

And the other way around too:

```rust,ignore
{{#include ../../../examples/book_serde.rs:ser_test}}
```

## One Way Only

A struct does not always need to be both serialized and deserialized. Which `serde` trait will be implemented for the struct is determined by the two sub-arguments of the `serde` argument, whose names are (surprise!) `serialize` and `deserialize`. For example, if we only want to serialize the struct, we simply write:

```rust,ignore
#[fxstruct(serde(serialize))]
```

Or

```rust,ignore
#[fxstruct(serde(deserialize(off)))]
```

```admonish tip
The general rule FieldX follows here is that if the `serde` argument is applied and not disabled with the `off` flag, then the user intends for one of the two traits to be implemented, whether the desired one is enabled or the undesired one is disabled.
```

## Default Value

It is possible to specify a default for deserialization alone, separate from the field's default value. In other words, it is possible to distinguish the default value a field receives during construction from the one it receives during deserialization if the field is omitted in the serialized data. For example:

```rust,ignore
{{#include ../../../examples/book_serde.rs:defaults_decl}}

{{#include ../../../examples/book_serde.rs:defaults_test}}
```
