<!-- markdownlint-disable-next-line MD041 -->
[![Rust](https://github.com/vrurg/fieldx/actions/workflows/fieldx.yml/badge.svg?branch=v0.2)](https://github.com/vrurg/fieldx/actions/workflows/fieldx.yml)
[![License](https://img.shields.io/github/license/vrurg/fieldx)](https://github.com/vrurg/fieldx/blob/main/LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/fieldx)](https://crates.io/crates/fieldx)

# FieldX v0.2.1

Helper crate for [`fieldx`][__link0] and any third-party crates that extend its functionality. Can be used either to extend
FieldX functionality or to implement your own proc-macros.

`fieldx` is heavily based on the [`darling`][__link1] crate, which greatly simplifies proc-macro development,
but also imposes some constraints on attribute argument syntax. This crate overcomes these limitations
and provides support for attribute kinds required to implement `fieldx`.

Here is a brief breakdown of what is provided:

* Support for nested arguments, i.e. those that look like `arg1("value", trigger, subarg(...))`.
* Support for syntax elements not covered by the `darling` crate, such as
  `some_type(crate::types::Foo)` and
  `error(crate::error::Error, crate::error::Error::SomeProblem("with details"))`[^tuple].
* A set of types implementing standard `fieldx` arguments like helpers or literal values.

[^tuple]: Here, the first argument of `error()`—`Error`—is an enum, and `SomeProblem` is one of its variants.

## Usage

Imagine we are implementing a field-level attribute `foo` using the [`darling::FromField`][__link2] trait, and we want it to
accept the following arguments:

* `trigger`: enables or disables certain functionality
* `action`: specifies a method with special meaning
* `comment`: accepts arbitrary text
* `vis`: indicates whether field-related code should be public, and if so, which kind of `pub` modifier to use

A field declaration may take the following form with the attribute:

```rust
    #[foo(
        trigger,
        action("method_name", private),
        comment("Whatever we consider useful."),
        vis(pub(crate))
    )]
    bar: usize,
```

For this, you’ll need the following declaration somewhere in your proc-macro implementation:

```rust
#derive(FromField)
#[darling(attributes(foo))]
struct FooField {
    // ... skipping some darling default fields ...

    trigger: Option<FXBool>,
    action: Option<FXHelper>,
    comment: Option<FXString>,
    vis: Option<FXSynValue<syn::Visibility>>,
}
```

That’s all; this crate will take care of implementing the arguments for you!

Read the [FieldX Object Manager][__link3] book for more details on how to use this crate.


 [__cargo_doc2readme_dependencies_info]: ggGkYW0CYXSEGwRoBq6PUYV7GzX-0tnRIVJvG-1wwfgZ7UMfG7mqB6miVhFIYXKEG5vrXMml2IPGG3uhsRWqCstKG73GlMjap6bYG1-imR4k2pzZYWSBgmdkYXJsaW5nZzAuMjAuMTE
 [__link0]: https://docs.rs/fieldx
 [__link1]: https://docs.rs/darling
 [__link2]: https://docs.rs/darling/0.20.11/darling/?search=FromField
 [__link3]: https://vrurg.github.io/fieldx/

# License

Licensed under [the BSD 3-Clause License](/LICENSE).
