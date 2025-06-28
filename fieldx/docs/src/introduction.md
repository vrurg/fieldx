<!-- markdownlint-disable MD033 MD041 -->
<span style="float:right">
<a href="https://github.com/vrurg/fieldx">
    <img src="./img/github.svg" alt="GitHub">
</a>
<a href="https://crates.io/crates/fieldx">
    <img src="https://img.shields.io/crates/v/fieldx.svg" alt="Crates.io">
</a>
<a href="https://docs.rs/fieldx/latest/fieldx/">
    <img src="https://docs.rs/fieldx/badge.svg" alt="Docs.rs">
</a>
<a href="https://github.com/vrurg/fieldx/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/vrurg/fieldx" alt="License">
</a>
</span>
<!-- markdownlint-disable MD033 MD041 -->

# Introduction

FieldX is a declarative object orchestrator that streamlines object and dependency management. Key features include:

- Lazy initialization of fields via builder methods, simplifying implicit dependency management
- Accessor and setter methods generation
- Inner mutability pattern for fields
- Sync, async, and plain (unsync) modes of operation
- Integration with serde for serialization and deserialization
- [Builder pattern](https://en.wikipedia.org/wiki/Builder_pattern) for object creation
- Post-build hooks for validation and adjustment
- Generic structs support
- Reference-counted objects
- And more!

The crate doesn't have a strictly outlined purpose and can be helpful in various scenarios, such as:

- Dependency manager implementation
- Implicit dependency management and automated construction and initialization for objects
- Automation of provisioning of object interfaces
- Simplifying integration of complex object management logic with serialization and deserialization
- ... and counting.

The functionality of FieldX can be extended by third-party crates. At the moment there is an experimental [fieldx_plus](https://crates.io/crates/fieldx_plus) crate that helps with implementing parent/child and application/agent relationships between objects.

To sum up, FieldX is well-suited for:

- general application development
- concurrent and asynchronous environments
- scenarios where object internals should remain encapsulated
- cases in which boilerplate for object management and initialization becomes tedious and annoying

## The Book Purpose

This book is intended to provide a comprehensive overview of the FieldX crate, its features, and how to use it effectively. What it is not intended for is to provide a complete reference for the crate. For the latter, please refer to the [FieldX documentation](https://docs.rs/fieldx/latest/fieldx/).
