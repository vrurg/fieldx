# The First Steps

As with any other crate, begin by adding `fieldx` to your `Cargo.toml` file:

```toml
[dependencies]
fieldx = { version = "0.2", features = ["sync"] }
```

```admonish info
A detailed list of crate feature flags is available in the [FieldX documentation](https://docs.rs/fieldx/latest/fieldx/attr.fxstruct.html#crate-features).
```

Next, annotate a struct with the `#[fxstruct]` macro:

```rust,ignore
{{#include ../../../examples/book_first_steps.rs:declaration}}
```

That's it! Now you can use it as follows:

```rust,ignore
{{#include ../../../examples/book_first_steps.rs:usage}}
```

Let's say the struct grows in size and complexity, and it's time to implement the [builder pattern](https://en.wikipedia.org/wiki/Builder_pattern). No problem! Simply add the `builder` attribute to the struct:

```rust,ignore
{{#include ../../../examples/book_first_steps.rs:builder_decl}}

{{#include ../../../examples/book_first_steps.rs:builder_usage}}
```
