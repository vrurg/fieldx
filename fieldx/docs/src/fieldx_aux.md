<!-- markdownlint-disable MD033 MD041 -->
<div style="width: 100%; text-align: right;">
    <a href="https://github.com/vrurg/fieldx/tree/main/fieldx_aux">
        <img src="./img/github-fieldx_aux.svg" alt="fieldx_aux GitHub Path">
    </a>
    <a href="https://crates.io/crates/fieldx_aux">
        <img src="https://img.shields.io/crates/v/fieldx_aux.svg" alt="Crates.io">
    </a>
    <a href="https://docs.rs/fieldx_aux/latest/fieldx_aux/">
        <img src="https://docs.rs/fieldx_aux/badge.svg" alt="Docs.rs">
    </a>
    <a href="https://github.com/vrurg/fieldx/blob/main/LICENSE">
        <img src="https://img.shields.io/github/license/vrurg/fieldx" alt="License">
    </a>
</div>

# FieldX Auxiliary Crate

In this chapter, we will discuss the {{i:`fieldx_aux`}} crate, the principles it is built upon, and how it can be used to extend FieldX or implement your own proc-macros.

## What is `fieldx_aux`?

As FieldX evolved from a simple wrapper around standard Rust primitives, implemented by a beginner rustacean, into the complex framework it is today, certain inner concepts crystallized into a set of types and traits. Along the way, the idea of implementing parent/child object relationships emerged. However, including this feature directly into FieldX was deemed excessive and conceptually unclear. Proper implementation of a new proc-macro required compatibility with the core FieldX code, as well as significant boilerplate. This led to the inevitable decision to extract a subset of inner interfaces and make them publicly available[^not_only].

[^not_only]: Eventually, it also became necessary to publish some inner logic of the FieldX core, resulting in the creation of the `fieldx_core` crate, which is covered in its own chapter in this book.

The `fieldx_aux` crate provides a loosely coordinated set of primitives, such as the type [`fieldx_aux::FXNestingAttr`], which implements nested function-like argument syntax like `serde(attributes_fn(derive(Debug)))`. Another example is [`fieldx_aux::FXHelper`], which supports helper arguments like `reader`, `writer`, or `clearer`. The coordination between these components is minimal; for instance, `FXHelper` is essentially a [`fieldx_aux::FXBaseHelper`] that implements the [`fieldx_aux::FromNestAttr`] trait, enabling it to be wrapped into the `FXNestingAttr` type.

In the following sections, we will explore some concepts and primitives provided by the `fieldx_aux` crate. The goal is to familiarize you with them, while more detailed information can be found in the [FieldX documentation](https://docs.rs/fieldx_aux/latest/fieldx_aux/).
