<!-- markdownlint-disable-next-line MD041 -->
[![Rust](https://github.com/vrurg/fieldx/actions/workflows/fieldx.yml/badge.svg?branch=v0.2)](https://github.com/vrurg/fieldx/actions/workflows/fieldx.yml)
[![License](https://img.shields.io/github/license/vrurg/fieldx)](https://github.com/vrurg/fieldx/blob/main/LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/fieldx)](https://crates.io/crates/fieldx)

# FieldX v0.2.1

FieldX is a declarative object orchestrator that streamlines object and dependency management. It supports:

* Lazy initialization of fields with builder methods that simplifies implicit dependency management
* Accessor and setter methods for fields
* Optional field infrastructure
* Sync-safe field management with locks
* Struct builder pattern
* Post-build hook for validation and adjustment of struct
* `serde` support
* Type conversions using `Into` trait
* Default values for fields
* Inner mutability for fields
* Pass-through attributes for fields, methods, and generated helper structs
* Renaming for generated methods names and serialization inputs/outputs
* Generic structs
* Visibility control for generated methods and helper structs

## Quick Start

Let’s start with an example:

```rust
use fieldx::fxstruct;

#[fxstruct(lazy)]
struct Foo {
    count: usize,
    foo:   String,
    // This declaration can be replaced with:
    //     #[fieldx(lazy(off), inner_mut, get, get_mut)]
    //     order: Vec<&'static str>,
    // But we want things here be a bit more explicit for now.
    #[fieldx(lazy(off), get)]
    order: RefCell<Vec<&'static str>>,
}

impl Foo {
    fn build_count(&self) -> usize {
        self.order.borrow_mut().push("Building count.");
        12
    }

    fn build_foo(&self) -> String {
        self.order.borrow_mut().push("Building foo.");
        format!("foo is using count: {}", self.count())
    }
}

let foo = Foo::new();
assert_eq!(foo.order().borrow().len(), 0);
assert_eq!(foo.foo(), "foo is using count: 12");
assert_eq!(foo.foo(), "foo is using count: 12");
assert_eq!(foo.order().borrow().len(), 2);
assert_eq!(foo.order().borrow()[0], "Building foo.");
assert_eq!(foo.order().borrow()[1], "Building count.");
```

What happens here is:

* A struct where all fields are `lazy` by default, meaning they are lazily initialized using corresponding
  `build_<field_name>` methods that provide the initial values.
* Laziness is explicitly disabled for the `order` field, meaning it will be initialized with its default value.

At run-time, we first ensure that the `order` vector is empty, i.e., none of the `build_` methods was called. Then
we read from `foo` using its accessor method, resulting in the field’s builder method being called. The method, in turn,
uses the `count` field via its accessor method, which also invokes `count`’s builder method.

Each builder method updates the `order` field with a message indicating that it was called. Then we make sure that
each `build_` method was invoked only once.

It must be noticeable that a minimal amount of handcraft is needed here as most of the boilerplate is handled by the `fxstruct` attribute,
which even provides a basic `new()` constructor for the struct.

## Further Reading

* The [FieldX Object Manager][__link0] book.
* Helper crates for 3rd-party extensions that are used by FieldX itself:
  * [`fieldx_aux`][__link1] implements a set of types.
  * [`fieldx_core`][__link2] implements the core functionality of FieldX.

## Feature Flags

The following feature flags are supported by this crate:

|*Feature*|*Description*|
|-------|-----------|
|**sync**|Support for sync-safe mode of operation|
|**async**|Support for async mode of operation|
|**tokio-backend**|Selects the Tokio backend for async mode. A no-op without the `async` feature.|
|**async-lock-backend**|Selects the `async-lock` backend for async mode. A no-op without the `async` feature.|
|**async-tokio**|Combines `async` and `tokio-backend` features.|
|**async-lock**|Combines `async` and `async-lock-backend` features.|
|**clonable-lock**|Enables the [clonable lock wrapper type][__link3].|
|**send_guard**|See corresponding feature of the [`parking_lot` crate][__link4]|
|**serde**|Enable support for `serde` marshalling.|
|**diagnostics**|Enable additional diagnostics for compile time errors. Experimental, requires Rust nightly toolset.|

**Note:** The `tokio-backend` and `async-lock-backend` features are mutually exclusive. You can only use one of them
at a time or FieldX will produce a compile-time error.


 [__link0]: https://vrurg.github.io/fieldx/
 [__link1]: https://docs.rs/fieldx_aux
 [__link2]: https://docs.rs/fieldx_core
 [__link3]: more_on_locks.md
 [__link4]: https://crates.io/crates/parking_lot



# License

Licensed under [the BSD 3-Clause License](/LICENSE).
