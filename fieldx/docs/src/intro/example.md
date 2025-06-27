# Example

This code focuses on lazy initialization of fields and shows how implicit dependencies work. Essentially, it forms the basis of a dependency manager or a large object requiring complex initialization logic.

A quick explanation:

- The `lazy` argument of the `fxstruct` macro declares that all fields of the struct are initialized lazily by default. The exception is the `order` field, which opts out using `lazy(off)`.
- The `count` field is assigned a default value of _42_. Thus, a newly created instance of `Foo` will have this value from the start, but since it’s lazy it can be reset to an uninitialized state with the `clear_count` method (introduced by the `clearer` attribute). The next read will initialize it by calling `build_count`.
    The `predicate` argument also provides a `has_count` method to check whether the field currently holds a value.
- The `comment` field is purely lazy, without a default value.

Because laziness means a field only gets its value when first accessed, all fields automatically receive accessor methods. The `count` field is special here: its accessor is explicitly requested with `get(copy)`, so instead of returning a reference (the usual behavior), it returns the `usize` value directly, which is more ergonomic since `usize` implements `Copy`.

The overall point of this sample is to demonstrate how the `comment` field gets its value constructed using the `count` field. As soon as the `count` changes, the `comment` gets a new value after re-initialization.

The example also logs each call to the corresponding builder methods of `count` and `comment` into the `order` field. By inspecting its content later, we can prove that the builders were only invoked when really needed, and that the order of invocation was determined by the initialization logic of the fields.

```rust,ignore
{{#include ../../../examples/book_intro_example.rs:decl}}

{{#include ../../../examples/book_intro_example.rs:main}}
```
