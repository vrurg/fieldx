# Builder Pattern

The {{i:builder pattern}} implemented with the `builder` argument added to the `fxstruct` macro or to one of the field `fieldx` attributes. The [First Steps](./first_steps.md) chapter already mentions this, so let's just borrow its example:

```rust ignore
{{#include ../../../examples/book_first_steps.rs:builder_decl}}

{{#include ../../../examples/book_first_steps.rs:builder_usage}}
```

There are two aspects worth mentioning here.

The first is the `builder(off)` argument of the `available` field. Apparently, with this flag, the field is not settable via the {{i:builder type}}. But since we still need a value to be put into the field, the only one we can use is the default one. For `u32`, it is `0`. If any other value is needed, it can be set via the `default` argument of the `fieldx` attribute:

```rust ignore
#[fieldx(get(copy), builder(off), default(123))]
```

The second aspect is the `.expect("...")` method call at the end of the builder chain. This way, we handle possible errors that may occur when a required value is not set. Imagine commenting out the `.year(1979)` call in the example above. This is a pure run-time error that the compiler cannot catch, and we must handle it ourselves at run time.

This brings us to the next topic, which is discussed in the [next chapter](./optional_values.md#builder). For now, we have a bit more to discuss here.

## Opt-in Approach {{hi:opt-in builder}}

For a big struct where only a couple of fields need to receive values from the user, it could be quite tedious to add `builder(off)` to each one that is not settable via the builder. There is a way in FieldX to make this easier: use the "opt-in" approach:

```rust ignore
{{#include ../../../examples/book_builder_pattern.rs:opt_in_decl}}

{{#include ../../../examples/book_builder_pattern.rs:opt_in_usage}}
```

This example isn't perfect because there are more buildable fields than non-buildable ones, but it demonstrates the point. Attempting to add an `.available(123)` call to the builder chain will result in a compile-time error.

The same result can be achieved by simply adding the `builder` argument to the fields where we want it. In this case, FieldX will imply that we want the opt-in scheme used for the struct. Why the `opt_in` sub-argument, you may ask? Sometimes one may want it for readability purposes, but more importantly, it allows specifying additional struct-level sub-arguments with the `builder` argument without resulting in the need to go the every-field-`builder(off)` route.

```rust ignore
{{#include ../../../examples/book_builder_pattern.rs:opt_in_subarg_decl}}

{{#include ../../../examples/book_builder_pattern.rs:opt_in_subarg_usage}}
```

In this snippet, we have two changes that can only be done at the struct level:

- We've given a new name to the builder type.
- We added a common prefix to the names of build {{i:setter method}}s.

## Default Value

When there is a default value specified for a field, FieldX takes it into account when generating the builder type. The implication it has is that the corresponding setter method for the field could be omitted from the builder chain. Sure enough, in this case, the field will be initialized with its default:

```rust ignore
{{#include ../../../examples/book_builder_pattern.rs:with_default_decl}}

{{#include ../../../examples/book_builder_pattern.rs:with_default_usage}}
```

```admonish tip title="BTW"
Here we also implement the above-mentioned approach where fields are given the `builder` argument and the struct-level `builder(opt_in)` is assumed.
```

## Coercion

Similar to the setter methods, the builder methods can also [coerce](./coercion.md) their arguments into the field type. As with the setters, this is achieved using the `into` sub-argument:

```rust ignore
{{#include ../../../examples/book_builder_pattern.rs:coerce_decl}}

{{#include ../../../examples/book_builder_pattern.rs:coerce_usage}}
```

This example demonstrates several concepts simultaneously:

- The use of `into` as the struct-level default.
- Field-level override for the default.
- Usage with an optional field.
