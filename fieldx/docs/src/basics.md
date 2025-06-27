# Basics

The `fieldx` module provides two attributes: `fxstruct`, which is the actual macro and configures named structs (no unions or enums are supported), and `fieldx`, which adjusts field parameters.

When applied, the `fxstruct` macro transforms the annotated struct based on its own arguments and any subsidiary `fieldx` attributes. Notable changes and additions may include, but are not limited to:

- Wrapping field types in container types (see [The Inner Workings](inner_workings.md)).
    In the [introduction example](intro/example.md), `comment` and `count` become
    [`OnceCell<String>`](crate::plain::OnceCell) and `OnceCell<usize>`, while `order` remains unchanged.

- Generating helper methods and associated functions for the struct. This includes accessor methods and the `new()` associated method.

- Implementing the `Default` trait.

- Generating a builder type and the `builder()` associated method.

- Generating a shadow struct for [serialization/deserialization](./basics/serialization.md) with [`serde`](https://serde.rs/).

The following chapters will introduce you into FieldX, covering basics of these and other topics.
