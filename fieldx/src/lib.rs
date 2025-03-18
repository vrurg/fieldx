#![doc(html_root_url = "https://docs.rs/")]
//! # FieldX
//!
//! `fieldx` is a declarative object orchestrator that streamlines object and dependency management. It supports:
//!
//! - Lazy initialization of fields with builder methods that simplifies implicit dependency management
//! - Accessor and setter methods for fields
//! - Optional field infrastructure
//! - Sync-safe field management with locks
//! - Struct builder pattern
//! - Post-build hook for validation and adjustment of struct
//! - `serde` support
//! - Type conversions using `Into` trait
//! - Default values for fields
//! - Inner mutability for fields
//! - Pass-through attributes for fields, methods, and generated helper structs
//! - Renaming for generated methods names and serialization inputs/outputs
//! - Generic structs
//! - Visibility control for generated methods and helper structs
//!
//! # Quick Start
//!
//! Let's start with an example:
//!
//! ```
//! # use std::cell::RefCell;
//! use fieldx::fxstruct;
//!
//! #[fxstruct(lazy)]
//! struct Foo {
//!     count: usize,
//!     foo:   String,
//!     // This declaration can be replaced with:
//!     //     #[fieldx(lazy(off), inner_mut, get, get_mut)]
//!     //     order: Vec<&'static str>,
//!     // But we want things here be a bit more explicit for now.
//!     #[fieldx(lazy(off), get)]
//!     order: RefCell<Vec<&'static str>>,
//! }
//!
//! impl Foo {
//!     fn build_count(&self) -> usize {
//!         self.order.borrow_mut().push("Building count.");
//!         12
//!     }
//!
//!     fn build_foo(&self) -> String {
//!         self.order.borrow_mut().push("Building foo.");
//!         format!("foo is using count: {}", self.count())
//!     }
//! }
//!
//! # fn main() {
//! let foo = Foo::new();
//! assert_eq!(foo.order().borrow().len(), 0);
//! assert_eq!(foo.foo(), "foo is using count: 12");
//! assert_eq!(foo.foo(), "foo is using count: 12");
//! assert_eq!(foo.order().borrow().len(), 2);
//! assert_eq!(foo.order().borrow()[0], "Building foo.");
//! assert_eq!(foo.order().borrow()[1], "Building count.");
//! # }
//! ```
//!
//! What happens here is:
//!
//! - a struct with all fields been `lazy` by default
//! - laziness is explicitly disabled for field `order`
//! - methods `build_count` and `build_foo` return initial values for corresponding fields
//!
//! At run-time we first ensure that the `order` vector is empty meaning none of the `build_` methods was called. Then
//! we read from `foo` using its accessor method. Then we make sure that each `build_` method was invoked only once.
//!
//! As one can notice, a minimal amount of handcraft is needed here as most of boilerplate is handled by the macro,
//! which provides even basic `new` associated function.
//!
//! Also notice that we don't need to remember the order of initialization of fields. Builder of `foo` is using `count`
//! without worrying if it's been initialized yet or not because it will always be.
//!
//! # Basic
//!
//! The module provides two attributes: `fxstruct`, and `fieldx`. The first is responsible for configuring structs, the
//! second for adjusting field parameters.
//!
//! The macro can only be used with named structures, no union types, nor enums are supported. When applied, it rewrites
//! the type it is applied to according to the parameters provided. Here is a list of most notable changes and
//! additions:
//!
//! - field types may be be wrapped into container types (see [The Inner Workings](#inner_workings))
//!
//!   In the above example `foo` and `count` become [`OnceCell<String>`][OnceCell] and `OnceCell<usize>`, whereas
//!   `order` remains unchanged.
//!
//! - a partial implementation of `Foo` is added with helper and special methods and associated functions ([Field Or
//! Method](#field_or_method) in this section)
//!
//!   I.e. this is where accessor methods and `new` live.
//!
//! - depending on parameters, an implicit implementation of the [`Default`] trait may be be added
//! - if requested, builder struct and `builder()` associated function will be implemented
//! - also, if requested, a shadow struct for correct `serde` support will be there too
//!
//! <a id="field_or_method"></a>
//! ## Field Or Method?
//!
//! Normally it is recommended to use module-generated helper methods to access, modify, or otherwise interact with
//! struct fields. Use of the methods provides both better code readability and, sometimes, better functionality. Like,
//! for example, marking a field as `#[fieldx(get(clone))` would always be returning a plain cloned instance of the
//! field value.
//!
//! But when there is a need to work with a field directly (for example, to implement own accessor with additional
//! functionality) `fieldx` provides own container types that are aimed at providing necessary API. See [`FXProxySync`]
//! and [`FXProxyAsync`] container types.
//!
//! # Sync, Async, And Plain Structs
//!
//! _Note:_ "Async" is considered synonymous with "sync" since both require concurrency safety. Even the code generated
//! for sync and async cases is mostly identical.
//!
//! If a thread-safe struct is needed then `fxstruct` must take the `sync` argument: `#[fxstruct(sync, ...)]`. When
//! instructed so, the macro will do its best to provide concurrency safety at the field level. It means that:
//!
//! - lazy builder methods are guaranteed to be invoked once and only once per each initialization, be it single- or
//!   multi-threaded application
//! - access to field is lock-protected for lazy fields implicitly
//!
//! In less strict cases it is possible to mark individual fields as sync.
//!
//! Plain non-mutable accessors normally return a reference to their field. Accessors of sync structs, unless directed
//! to use [`clone`][`Clone`] or [`copy`][`Copy`], or used with a non-protected field, return some kind of lock-guard
//! object.
//!
//! Wrapper types for sync struct fields are non-`std` and provided with the module.
//!
//! <a id="protected_unprotected_fields"></a>
//! ## Protected And Unprotected Fields Of Sync Structs
//!
//! For a `fieldx` sync struct to be `Sync+Sent` all of its fields are expected to be _lock-protected_ (or, sometimes we
//! could just say _"protected"_). But "expected" doesn't mean "has to be". Unless defaults, specified with `fxstruct`
//! attribute (i.e. with _struct-level_ arguments) tell otherwise, fields not marked with `fieldx` attribute with
//! corresponding arguments will remain _unprotected_. I.e.:
//!
//! ```ignore
//! #[fxstruct(sync)]
//! struct Foo {
//!     #[fieldx(lazy)]
//!     foo: String, // protected
//!     #[fieldx(get_mut)]
//!     bar: String, // unprotected
//! }
//! ```
//!
//! Of course, whether the struct remains thread-safe would then depend on the safety of unprotected fields.
//!
//! <a id="optional_fields"></a>
//! # Optional Fields
//!
//! _Optional_ in this context has the same meaning, as in the [`Option`] type. Sure thing, one can simply declare a
//! field using the core type (and, as a matter of fact, this is what `fieldx` is using internally anyway). What's the
//! advantages of using `fieldx` then?
//!
//! First of all, manual declaration may mean additional boilerplate code to implement an accessor, among other things.
//! With `fieldx` most of it can be hidden under a single declaration:
//!
//! <a id="optional_example"></a>
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct]
//! struct Foo {
//!     #[fieldx(predicate, clearer, get, set(into))]
//!     description: String,
//! }
//!
//! # fn main() {
//! let mut obj = Foo::new();
//! assert!( !obj.has_description() );
//! obj.set_description("foo");
//! assert!( obj.has_description() );
//! assert_eq!( obj.description(), &Some(String::from("foo")) );
//! obj.clear_description();
//! assert!( !obj.has_description() );
//! # }
//! ```
//!
//! _`<digression_mode>`_ Besides, aesthetically, to some `has_description` is more appealing than
//! `obj.description().is_some()`. _`</digression_mode>`_
//!
//! Next, optional fields of `sync` structs are lock-protected by default. This can be changed with explicit
//! `lock(off)`, but one has to be aware that then sync status of the struct will depend the safety of the field.
//!
//! And the last note to be made is that if at some point it would prove to be useful to convert a field into a `lazy`
//! then refactoring could be reduced to simply adding corresponding argument the `fieldx` attribute and implementing a
//! new builder for it.
//!
//! # Laziness Protocol
//!
//! Though being very simple concept, laziness has its own peculiarities. The basics, as shown above, are such that when
//! we declare a field as `lazy` the macro wraps it into some kind of proxy container type ([`OnceCell`] for plain
//! fields). The first read[^only_via_method] from an uninitialized field will result in the lazy builder method to be
//! invoked and the value it returns to be stored in the field.
//!
//! Here come the caveats:
//!
//! 1. A builder is expected to be infallible. This requirement comes from the fact that when we call field's accessor
//!    we expect a value of field's type to be returned. Since Rust requires errors to be handled semi-in-place (contrary
//!    to exceptions in many other languages) there is no way for us to overcome this limitation. The builder could panic,
//!    but this is rarely a good option.
//!
//!    For cases when it is important to have controllable error handling, one could give the field a [`Result`] type.
//!    Then `obj.field()?` could be a way to take care of errors. But this approach has its own complications,
//!    especially for sync fields.
//!
//! 1. Field builder methods cannot mutate their objects. This limitation also comes from the fact that a typical
//!    accessor method doesn't need and must not use mutable `&self`. Of course, it is always possible to use internal
//!    mutability, as in the first example here.
//!
//! [^only_via_method]: Apparently, the access has to be made by calling a corresponding method. Mostly it'd be field's
//! accessor, but for `sync` structs it's more likely to be a reader.
//!
//! # Field Interior Mutability
//!
//! Marking fields with `inner_mut` flag is a shortcut for using [`RefCell`] wrapper. This effectively turns such fields
//! to be plain ones.
//!
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct]
//! struct Foo {
//!     #[fieldx(inner_mut, get, get_mut, set, default(String::from("initial")))]
//!     modifiable: String,
//! }
//!
//! # fn main() {
//! let foo = Foo::new();
//! let old = foo.set_modifiable(String::from("manual"));
//! assert_eq!(old, String::from("initial"));
//! assert_eq!(*foo.modifiable(), String::from("manual"));
//! *foo.modifiable_mut() = String::from("via mutable accessor");
//! assert_eq!(*foo.modifiable(), String::from("via mutable accessor"));
//! # }
//! ```
//!
//! Note that this pattern is only useful when the field must not be neither optional nor lock-protected in
//! `sync`-declared structs.
//!
//! # Builder Pattern
//!
//! **IMPORTANT!** First of all, it is necessary to mention unintended terminological ambiguity here. The terms `build`
//! and `builder` are used for different, though identical in nature, processes. As mentioned in the previous section,
//! the _lazy builders_ are methods that return initial values for associated fields. The _struct builder_ in this
//! section is an object that collects initial values from user and then is able to create the final instance of the
//! original struct.  This ambiguity has some history spanning back to the times when Perl's
//! [`Moo`](https://metacpan.org/pod/Moo) module was one of the author's primary tools. Then it was borrowed by Raku
//! [`AttrX::Mooish`](https://raku.land/zef:vrurg/AttrX::Mooish) and, finally, automatically made its way into `fieldx`
//! which, initially, didn't implement the builder pattern.
//!
//! The default `new` method generated by `fxstruct` macro accepts no arguments and simply creates a bare-bones object
//! initialized from type defaults. Submitting custom values for struct fields is better be done by using the
//! builder pattern:
//!
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct(builder)]
//! struct Foo {
//!     #[fieldx(lazy)]
//!     description: String,
//!     count: usize,
//! }
//!
//! impl Foo {
//!     fn build_description(&self) -> String {
//!         format!("this is item #{}", self.count)
//!     }
//! }
//!
//! # fn main() {
//! let obj = Foo::builder()
//!             .count(42)
//!             .build()
//!             .expect("Foo builder failure");
//! assert_eq!( obj.description(), &String::from("this is item #42") );
//!
//! let obj = Foo::builder()
//!             .count(13)
//!             .description(String::from("count is ignored"))
//!             .build()
//!             .expect("Foo builder failure");
//! // Since the `description` is given a value the `count` field is not used
//! assert_eq!( obj.description(), &String::from("count is ignored") );
//! # }
//! ```
//!
//! Since the only `fieldx`-related failure that may happen when building a new object instance is a required field not
//! given a value, the `build()` method would return [`FieldXError`](error::FieldXError) if this happens.
//!
//! # Crate Features
//!
//! The following featues are supported by this crate:
//!
//! | *Feature* | *Description* |
//! |-|-|
//! | `sync` | Support for sync-safe mode of operation |
//! | `async` | Support for async mode of operation |
//! | `serde` | Enable support for `serde` marshalling. |
//! | `send_guard` | See corresponding feature of the [`parking_lot` crate](https://crates.io/crates/parking_lot) |
//! | `diagnostics` | Enable additional diagnostics for compile time errors. Requires Rust nightly toolset. |
//!
//! # Usage
//!
//! Most arguments of both `fxstruct` and `fieldx` can take either of the two forms: a keyword (`arg`), or a
//! *"function"* (`arg(subarg)`).
//!
//! Also, most of the arguments are shared by both `fxstruct` and `fieldx`. But their meaning and the way their
//! arguments are interpreted could be slightly different for each attribute. For example, if an argument takes a
//! literal string sub-argument it is likely to be a method name when associated with `fieldx`; but for `fxstruct` it
//! would define common prefix for method names.
//!
//! There is also a commonality between most of the arguments: they can be temporarily (say, for testing purposes) or
//! permanently turned off by using `off` sub-argument with them. See `lazy(off)` in the
//! above example.
//!
//!
//! <a id="about_default"></a>
//! # Do We Need The `Default` Trait?
//!
//! Unless explicit `default` argument is used with the `fxstruct` attribute, `fieldx` tries to avoid implementing the
//! `Default` trait unless really required. Here is the conditions which determine if the implementation is needed:
//!
//! 1. Method `new` is generated by the procedural macro.
//!
//!    This is, actually, the default behavior which is disabled with [`no_new`](#no_new) argument of the `fxstruct`
//!    attribute.
//! 1. A field is given a [`default`](#default) value.
//! 1. The struct is `sync` and has a lazy field.
//!
//! <a id="accessor_vs_reader_writer"></a>
//! # Why `get`/`get_mut` and `reader`/`writer` For Sync Structs?
//!
//! It may be confusing at first as to why there are, basically, two different kinds of accessors for sync structs. But
//! there are reasons for it.
//!
//! First of all, let's take into account these important factors:
//!
//! - fields, that are [protected](#protected_unprotected_fields), cannot provide their values directly; lock-guards are
//!   required for this
//! - lazy fields are expected to always get some value when read from
//!
//! Let's focus on a case of lazy fields. They have all properties of lock-protected and optional fields, so we loose
//! nothing in the context of the `get`/`get_mut` and `reader`/`writer` differences.
//!
//! ## `get` vs `reader`
//!
//! A bare bones `get` accessor helper is the same thing, as the `reader` helper[^get_reader_guts]. But, as soon as a
//! user decides that they want `copy` or `clone` accessor behavior, `reader` becomes the only means of reaching out
//! to field's lock-guard:
//!
//! [^get_reader_guts]: As a matter of fact, internally they even use the same method-generation code.
//!
//! ```
//! # use fieldx::fxstruct;
//! # fn main() {
//! # #[cfg(feature = "sync")]
//! # {
//! #[fxstruct(sync)]
//! struct Foo {
//!     #[fieldx(get(copy), reader, lazy)]
//!     bar: u32
//! }
//! impl Foo {
//!     fn build_bar(&self) -> u32 { 1234 }
//!     fn do_something(&self) -> u32 {
//!         // We need to protect the field value until we're done using it.
//!         let bar_guard = self.read_bar();
//!         let outcome = *bar_guard * 2;
//!         outcome
//!     }
//! }
//! let foo = Foo::new();
//! assert_eq!(foo.do_something(), 2468);
//! # }
//! # }
//! ```
//!
//! ## `get_mut` vs `writer`
//!
//! This case if significantly different. Despite both helpers are responsible for mutating fields, the `get_mut` helper
//! remains an accessor in first place, whereas the `writer` is not. In the context of lazy fields it means that
//! `get_mut` guarantees the field to be initialized first. Then we can mutate its value.
//!
//! `writer`, instead, provides direct and immediate access to the field's container. It allows to store a value into it
//! without the builder method to be involved. Since building a lazy field can be expensive, it could be helpful to
//! avoid it in cases when we don't actually need it[^sync_writer_vs_builder].
//!
//! [^sync_writer_vs_builder]: Sometimes, if the value is known before a struct instance is created, it might make sense
//! to use the builder instead of the writer.
//!
//! Basically, the guard returned by the `writer` helper can only do two things: store an entire value into the field,
//! and clear the field.
//!
//! ```
//! # use fieldx::fxstruct;
//! # fn main() {
//! # #[cfg(feature = "sync")]
//! # {
//! #[fxstruct(sync)]
//! struct Foo {
//!     #[fieldx(get_mut, get(copy), writer, lazy)]
//!     bar: u32
//! }
//! impl Foo {
//!     fn build_bar(&self) -> u32 {
//!         eprintln!("Building bar");
//!         1234
//!     }
//!     fn do_something1(&self) {
//!         eprintln!("Using writer.");
//!         let mut bar_guard = self.write_bar();
//!         bar_guard.store(42);
//!     }
//!     fn do_something2(&self) {
//!         eprintln!("Using get_mut.");
//!         let mut bar_guard = self.bar_mut();
//!         *bar_guard = 12;
//!     }
//! }
//!
//! let foo = Foo::new();
//! foo.do_something1();
//! assert_eq!(foo.bar(), 42);
//!
//! let foo = Foo::new();
//! foo.do_something2();
//! assert_eq!(foo.bar(), 12);
//! # }
//! # }
//! ```
//!
//! This example is expected to output something like this:
//!
//! ```ignore
//! Using writer.
//! Using get_mut.
//! Building bar
//! ```
//!
//! As you can see, use of the `bar_mut` accessor results in the `build_bar` method invoked.
//!
//! <a id="inner_workings"></a>
//! # The Inner Workings
//!
//! As it was mentioned in the [Basics](#basics) section, `fieldx` rewrites structures with `fxstruct` applied. The
//! following table reveals the final types of fields. `T` in the table represents the original field type, as specified
//! by the user; `O` is the original struct type.
//!
//! | Field Parameters | Plain Type | Sync Type | Async Type |
//! |------------------|---------------|-----------|-----------|
//! | `lazy` | `OnceCell<T>` | [`FXProxySync<O, T>`] | [`FXProxyAsync<O,T>`] |
//! | `optional` (also activated with `clearer` and `proxy`) | `Option<T>` | [`FXRwLockSync<Option<T>>`][`sync::FXRwLockSync`] | [`FXRwLockAsync<Option<T>>`][`async::FXRwLockAsync`] |
//! | `lock`, `reader` and/or `writer` | N/A | [`FXRwLockSync<T>`][`sync::FXRwLockSync`] | [`FXRwLockAsync<T>`][`async::FXRwLockAsync`] |
//!
//! Apparently, skipped fields retain their original type. Sure enough, if such a field is of non-`Send` or non-`Sync`
//! type the entire struct would be missing these traits despite all the efforts from the `fxstruct` macro.
//!
//! There is also a difference in how the initialization of `lazy` fields is implemented. For plain fields this is done
//! directly in their accessor methods. Sync structs delegate this functionality to the [`FXProxySync`] type.
//!
//! ## Traits
//!
//! `fieldx` additionally implement traits `FXStructNonSync` and `FXStructSync` for corresponding kind of structs. Both
//! traits are empty and only used to distinguish structs from non-`fieldx` ones and from each other. For both of them
//! `FXStruct` is a super-trait.
//!
//! ## Sync Primitives
//!
//! The functionality of `sync` structs are backed by primitives provided by the [`parking_lot`] crate.
//!
//! # Support Of De-/Serialization With `serde`
//!
//! Transparently de-/serializing container types is a non-trivial task. Luckily, [`serde`] allows us to use special
//! parameters [`from`](https://serde.rs/container-attrs.html#from) and
//! [`into`](https://serde.rs/container-attrs.html#into) to perform indirect marshalling via a shadow struct. The way
//! this functionality implemented by `serde` (and it is for a good reason) requires our original struct to implement
//! the [`Clone`] trait. `fxstruct` doesn't automatically add a `#[derive(Clone)]` because implementing the trait
//! might require manual work from the user.
//!
//! Normally one doesn't need to interfere with the marshalling process. But if such a need emerges then the following
//! implementation details might be helpful to know about:
//!
//! - shadow struct mirror-fields of lazy and optional originals are [`Option`]-wrapped
//! - the struct may be given a custom name using string literal sub-argument of [the `serde` argument](#serde_struct)
//! - a shadow field may share its attributes with the original if they are listed in `forward_attrs` sub-argument of
//!   the `serde` argument
//! - `forward_attrs` is always applied to the fields, no matter if it is used with struct- or field-level `serde`
//!   argument
//! - if you need custom attributes applied to the shadow struct, use the `attributes*`-family of `serde` sub-arguments
//! - same is about non-shared field-level custom attributes: they are to be declared with field-level `attributes*` of
//!   `serde`
//!
//! [`parking_lot`]: https://docs.rs/parking_lot
//! [`serde`]: https://docs.rs/serde

#[cfg(feature = "async")]
pub mod r#async;
pub mod error;
#[cfg(feature = "sync")]
pub mod sync;
pub mod traits;

#[doc(hidden)]
pub use fieldx_aux::FXOrig;
pub use fieldx_derive::fxstruct;
#[doc(hidden)]
pub use once_cell::unsync::OnceCell;
#[cfg(feature = "async")]
pub use r#async::FXProxyAsync;
#[doc(hidden)]
pub use std::{
    cell::{Ref, RefCell, RefMut},
    fmt,
    sync::atomic::Ordering,
};
#[cfg(feature = "sync")]
pub use sync::FXProxySync;
