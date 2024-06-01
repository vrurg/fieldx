#![doc(html_root_url = "https://docs.rs/")]
//! Procedural macro for constructing structs with lazily initialized fields, builder pattern, and [`serde`] support
//! with a focus on declarative syntax.
//!
//! Let's start with an example:
//!
//! ```
//! # use std::cell::RefCell;
//! use fieldx::fxstruct;
//!
//! #[fxstruct( lazy )]
//! struct Foo {
//!     count: usize,
//!     foo:   String,
//!     #[fieldx( lazy(off), get )]
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
//! # Basics
//!
//! The module provides two attributes: `fxstruct`, and `fieldx`. The first is responsible for configuring structs, the
//! second for adjusting field parameters.
//!
//! The macro can only be used with named structures, no union types, nor enums are supported. When applied, it rewrites
//! the type it is applied to according to the parameters provided. Here is a list of most notable changes and
//! additions:
//!
//! - field types may be be wrapped into container types
//!
//!   In the above example `foo` and `count` become [`OnceCell<String>`][OnceCell] and `OnceCell<usize>`, whereas
//!   `order` remains unchanged.
//!
//! - a partial implementation of `Foo` is added with support methods and associated functions
//!
//!   I.e. this is where accessor methods and `new` live.
//!
//! - depending on parameters, an implicit implementation of the [`Default`] trait may be be added
//! - if requested, builder struct and `builder()` associated function will be implemented
//! - also, if requested, a shadow struct for correct `serde` support will be there too
//!
//! **Note** that user is highly discouraged from directly accessing modified fields. The module does its best to
//! provide all necessary API via corresponding methods.
//!
//! # Sync And Non-Sync Structs
//!
//! If a thread-safe struct is needed then `fxstruct` must take the `sync` argument: `#[fxstruct(sync, ...)]`. When told
//! so, the macro will do its best to provide concurrency safety at the field level. It means that:
//!
//! - builder methods are guaranteed to be invoked once and only once per each lazy initialization, be it single- or
//!   multi-threaded application
//! - access to struct fields is lock-protected (unless otherwise requested by the user)
//!
//! Sync and non-sync structures also are very different in ways they act and interact with user code. For example,
//! there is no way to have a mutable accessor for a sync structure.
//!
//! Also, non-mutable accessors of non-sync struct normally return a reference to their field. Accessors of sync structs
//! return either a [clone][`Clone`] or a [copy][`Copy`] of field value. Direct access to field value is provided via
//! lock-returning reader and writer methods (usually prefixed with `read_` and `write_`).
//!
//! Wrapper types for sync struct fields are non-`std` and provided with the module.
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
//! Next, optional fields of `sync` structs are automatically lock-protected.
//!
//! And the last note to be made is that if at some point it would prove to be useful to convert a field into a `lazy`
//! then refactoring could be reduced to simply adding corresponding argument the `fieldx` attribute and implementing a
//! new builder for it.
//!
//! # Laziness Protocol
//!
//! Though being very simple concept, laziness has its own peculiarities. The basics, as shown above, are such that when
//! we declare a field as `lazy` the macro wraps it into some kind of proxy container type ([`OnceCell`] for non-sync
//! structs). The first read[^only_via_method] from an uninitialized field will result in the builder method to be
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
//!    Then `obj.field()?` could be a way to take care of errors.
//!
//! 1. Field builder methods cannot mutate their objects. This limitation also comes from the fact that a typical
//!    accessor method doesn't need and must not use mutable `&self`. Of course, it is always possible to use internal
//!    mutability, as in the first example here.
//!
//! [^only_via_method]: Apparently, the access has to be made by calling a corresponding method. Mostly it'd be field's
//! accessor, but for `sync` structs it's more likely to be a reader.
//!
//! # Builder Pattern
//!
//! **IMPORTANT!** First of all, it is necessary to point out at unintended terminological ambiguity here. The terms
//! `build` and `builder` are used for different, though identical in nature, processes. The _lazy builders_ from the
//! previous section are methods that return initial values for associated fields. The _struct builder_ in this section
//! is an object that collects initial values from user and then is able to create the final instance of the original
//! struct. This ambiguity has some history spanning back to the times when Perl's [`Moo`](https://metacpan.org/pod/Moo)
//! module was one of the author's primary tools. Then it was borrowed by Raku
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
//! given a value, the `build()` method would return [`FieldXError`](errors::FieldXError) if this happens.
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
//! # Attribute Arguments
//!
//! <a id="attr_terminology"></a>
//! A few words on terminology:
//!
//! - argument **Type** determines what sub-arguments can be received:
//!   * _keyword_ – boolean-like, only accepts `off`: `keyword(off)`
//!   * _flag_ – similar to the _keyword_ above but takes no arguments; as a matter of fact, the `off` above is a _flag_
//!   * _helper_ - introduce functionality that is bound to a helper method (see below)
//!   * _list_ or _function_ – can take multiple sub-arguments
//!   * _meta_ - can take some syntax constructs
//! - helper method – implements certain functionality
//!
//!   Almost all helpers are generated by the macro. The only exception are lazy builders which must be provided by the
//!   user.
//! - **For** specifies if argument is specific to an attribute
//!
//! <a id="sub_args"></a>
//! ## Sub-Arguments of Helper Arguments
//!
//! Helper arguments share a bunch of common sub-arguments. We will describe them here, but if their meaning is unclear
//! it'd be better to skip this section and get back to it later.
//!
//! | Sub-argument | In fxstruct | In fxfield |
//! |-|-|-|
//! | **`off`** | disable helper | disable helper |
//! | a non-empty string literal (**`"foo"`**) | method name prefix | explicit method name (prefix not used) |
//! | **`attributes_fn`** | default attributes for corresponding kind of helper methods | attributes for field's helper method |
//! | <a id="visibility"></a> **`public`, `public(crate)`, `public(super)`, `public(some::module)`, `private`** | default visibility | visibility for field helper |
//!
//! For example:
//!
//! ```ignore
//! #[fxstruct( get( "get_", public(crate) ) )]
//! ```
//!
//! will generate accessor methods with names prefixed with `get_` and visibility `pub(crate)`:
//!
//! ```ignore
//! let foo = obj.get_foo();
//! ```
//!
//! With:
//!
//! ```ignore
//! #[fieldx( get( "special_type", private ) )]
//! ty: String,
//! ```
//!
//! a method of the field owning struct can use the accessor as follows:
//!
//! ```ignore
//! let foo = self.special_type();
//! ```
//!
//! <a id="attrs_family"></a>
//! ## `attributes*` Family of Sub-Arguments
//!
//! Sometimes it might be necessary to specify attributes for various generated syntax elements like methods, or
//! auxiliary structs. Where applicable, this functionality is supported by `attributes*` (sub)arguments. Their syntax
//! is `attributes(<attr1>, <attr2>, ...)` where an `<attr>` is specified exactly, as it would be specified in the code,
//! but with starting `#[` and finishing `]` being omitted.
//!
//! For example, `attributes_fn(allow(dead_code), cfg(feature = "myfeature"))` will expand into something like:
//!
//! ```ignore
//! #[allow(dead_code)]
//! #[cfg(feature = "myfeature")]
//! ```
//!
//! The following members of the family are currently supported: `attributes`, `attributes_fn`, and `attributes_impl`.
//! Which ones are supported in a particular context is documented below.
//!
//! ## Arguments of `fxstruct`
//!
//! ### **`sync`**
//!
//! **Type**: keyword
//!
//! Declare a struct as thread-safe.
//!
//! ### **`lazy`**
//!
//! **Type**: helper
//!
//! Enables lazy mode for all fields except those marked with `lazy(off)`.
//!
//! ### **`builder`**
//!
//! **Type**: helper
//!
//! Enables builder functionality by introducing a `builder()` associated function and builder type:
//!
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct(builder, get)]
//! struct Foo {
//!     description: String,
//! }
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let obj = Foo::builder()
//!                .description(String::from("some description"))
//!                .build()?;
//! assert_eq!(obj.description(), "some description");
//! # Ok(())
//! # }
//! ```
//!
//! Literal string sub-argument of `builder` defines common prefix for methods-setters of the builder. For example, with
//! `builder("set_")` one would then use `.set_description(...)` call.
//!
//! Additional sub-arguments:
//!
//! - **`attributes`** (see the [section above](#attrs_family)) – builder struct attributes
//! - **`attributes_impl`** - attributes of the struct implementation
//! - **`into`** – force all builder setter methods to attempt automatic type conversion using `.into()` method
//!
//!   With `into` the example above wouldn't need `String::from` and the call could look like this:
//!   `.description("some description")`
//!
//! ### **`no_new`**
//!
//! **Type**: keyword
//!
//! Disable generation of method `new`. This is useful for cases when a user wants their own `new` method.
//!
//! With this option the macro may avoid generating `Default` implementation for the struct. More details in [a section
//! below](#about_default).
//!
//! ### **`default`**
//!
//! **Type**: keyword
//!
//! Forces the `Default` implementation to be generated for the struct.
//!
//! ### **`get`**
//!
//! **Type**: helper
//!
//! Enables or disables getter methods for all fields, unless a field is marked otherwise.
//!
//! Additionally to the standard helper arguments accessors can also be configured as:
//!
//! - **`clone`** - cloning, i.e. returning a clone of the field value (must implement [`Clone`])
//! - **`copy`** - copying, i.e. returning a copy of the field value (must implement [`Copy`])
//! - **`as_ref`** – only applicable if field value is optional; it makes the accessor to return an `Option<&T>`
//!   instead of `&Option<T>`
//!
//! ### **`get_mut`**
//!
//! **Type**: helper
//!
//! Request for a mutable accessor. Since neither of additional options of `get` are applicable here[^no_copy_for_mut]
//! only basic [helper sub-arguments](#sub_args) are accepted.
//!
//! Mutable accessors have the same name, as immutable ones, but with `_mut` suffix, unless given explicit name by the
//! user:
//!
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct(get, get_mut)]
//! struct Foo {
//!     description: String,
//! }
//! # fn main() {
//! let mut obj = Foo::new();
//! *obj.description_mut() = "some description".to_string();
//! assert_eq!(obj.description(), "some description");
//! # }
//! ```
//!
//! **Important!** Mutable accessors are not possible for `sync` structs.
//!
//! [^no_copy_for_mut]: What sense is in having a mutable copy if you own it already?
//!
//! ### **`set`**
//!
//! **Type**: helper
//!
//! Request for setter methods. If a literal string sub-argument is supplied it is used as setter method prefix instead
//! of the default `set_`.
//!
//! Takes an additional sub-argument:
//!
//! - **`into`**: use the [`Into`] trait to automatically convert a value into the field type
//!
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct(set(into), get)]
//! struct Foo {
//!     description: String,
//! }
//! # fn main() {
//! let mut obj = Foo::new();
//! obj.set_description("some description");
//! assert_eq!(obj.description(), &"some description".to_string());
//! # }
//! ```
//!
//! <a id="reader_writer_helpers"></a>
//! ### **`reader`**, **`writer`**
//!
//! **Type**: helper
//!
//! Only meaningful for `sync` structs. Request for reader and writer methods that would return either read-only or
//! read-write lock guards. This is the only valid way to directly access field value in a concurrent environment.
//!
//! Akin to setters, method names are formed using `read_` and `write_` prefixes, correspondingly, prepended to the
//! field name.
//!
//! ```
//! # use fieldx::fxstruct;
//! #[fxstruct(sync, reader, writer)]
//! struct Foo {
//!     description: String,
//! }
//! # fn main() {
//! let obj = Foo::new();
//! {
//!     let mut wguard = obj.write_description();
//!     *wguard = String::from("let's use something different");
//! }
//! {
//!     let rguard = obj.read_description();
//!     assert_eq!(*rguard, "let's use something different".to_string());
//! }
//! # }
//! ```
//!
//! These helper are the primary means of accessing field content for `sync` structs. Writers are the only way to change
//! the field.
//!
//! ### **`clearer`** and **`predicate`**
//!
//! **Type**: helper
//!
//! These two are tightly coupled by their meaning, though can be used separately.
//!
//! Predicate helper methods return [`bool`] and are the way to find out if a field is set. They're universal in the way
//! that no matter wether a struct is sync or non-sync, or a field is lazy or just optional – you always use the same
//! method.
//!
//! Clearer helpers are the way to reset a field into uninitialized state. For optional fields it would simply mean it
//! will contain [`None`]. A lazy field would be re-initialized the next time it is read from.
//!
//! Clearers return the current field value. If field is already uninitialized (or never has been yet) `None` will be
//! given back.
//!
//! Using either of the two automatically make fields optional unless lazy.
//!
//! Check out the [example](#optional_example) in the [Optional Fields](#optional_fields) section.
//!
//! ### **`optional`**
//!
//! **Type**: keyword
//!
//! Explicitly make all fields optional. Useful when neither predicate nor clearer helpers are needed.
//!
//! ### **`public(...)`**, **`private`**
//!
//! Specify defaults for helpers. See [the sub-arguments section](#sub_args) above for more details.
//!
//! ### **`clone`**, **`copy`**
//!
//! Specify defaults for accessor helpers.
//!
//! <a id="serde_struct"></a>
//! ### **`serde`**
//!
//! **Type**: [function](#attr_terminology)
//!
//! Enabled with `serde` feature, which is off by default.
//!
//! Support for de/serialization will be discussed in more details in a section below. What is important to know at this
//! point is that due to use of container types direct serialization of a struct is hardly possible. Therefore `fieldx`
//! utilizes `serde`'s `into` and `from` by creating a special shadow struct. The shadow, by default, is named after the
//! original by prepending the name with double underscore and appending *Shadow* suffix: `__FooShadow`.
//!
//! The following sub-arguments are supported:
//!
//! - a string literal is used to give the shadow struct a user-specified name
//! - **`off`** disables de/serialization support altogether
//! - **`attributes(...)`** - custom [attributes](#attrs_family) to be applied to the shadow struct
//! - **`public(...)`**, **`private`** – specify [visibility](#visibility) of the shadow struct
//! - **`serialize`** - enable or disable (`serialize(off)`) serialization support for the struct
//! - **`deserialize`** - enable or disable (`deserialize(off)`) deserialization support for the struct
//! - **`default`** - wether `serde` must use defaults for missing fields and, perhaps, where to take the defaults from\
//! - **`forward_attrs`** - a list of field attributes that are to be forwarded to the corresponding field of the shadow
//!   struct
//!
//! ##### _Notes about `default`_
//!
//! Valid arguments for the sub-argument are:
//!
//! * a string literal that has the same meaning as for
//!   [the container-level `serde` attribute `default`](https://serde.rs/container-attrs.html#default--path)
//! * a path to a symbol that is bound to an instance of our type: `my_crate::FOO_DEFAULT`
//! * a call-like path that'd be used literally: `Self::serde_default()`
//!
//! The last option is preferable because `fieldx` will parse it and replace any found `Self` reference with the
//! actual structure name making possible future renaming of it much easier.
//!
//! There is a potentially useful "trick" in how `default` works. Internally, whatever type is returned by the
//! sub-argument it gets converted into the shadow type with trait [`Into`]. This allows you to use the original struct
//! as the trait implementation is automatically generated for it. See this example from a test:
//!
//! ```
//! #[cfg(feature = "serde")]
//! # mod inner {
//! # use fieldx::fxstruct;
//! # use serde::{Serialize, Deserialize};
//! #[fxstruct(sync, get, serde("BazDup", default(Self::serde_default())))]
//! #[derive(Clone)]
//! pub(super) struct Baz {
//!     #[fieldx(reader)]
//!     f1: String,
//!     f2: String,
//! }
//!
//! impl Baz {
//!     fn serde_default() -> Fubar {
//!         Fubar {
//!             postfix: "from fubar".into()
//!         }
//!     }
//! }
//!
//! struct Fubar {
//!     postfix: String,
//! }
//!
//! impl From<Fubar> for BazDup {
//!     fn from(value: Fubar) -> Self {
//!         Self {
//!             f1: format!("f1 {}", value.postfix),
//!             f2: format!("f2 {}", value.postfix),
//!         }
//!     }
//! }
//! # } // mod inner
//! # #[cfg(feature = "serde")]
//! # use inner::Baz;
//!
//! # fn main() {
//! # #[cfg(feature = "serde")]
//! # {
//! let json_src = r#"{"f1": "f1 json"}"#;
//! let foo_de = serde_json::from_str::<Baz>(&json_src).expect("Bar deserialization failure");
//! assert_eq!(*foo_de.f1(), "f1 json".to_string());
//! assert_eq!(*foo_de.f2(), "f2 from fubar".to_string());
//! # }
//! # }
//! ```
//!
//! ## Arguments of `fieldx`
//!
//! At this point, it's worth refreshing your memory about [sub-arguments of helpers](#sub_args) and how they differ in
//! semantics between `fxstruct` and `fieldx` attributes.
//!
//! ### **`skip`**
//!
//! **Type**: flag
//!
//! Leave this field alone. The only respected argument of `fieldx` when skipped is the `default`.
//!
//! ### **`lazy`**
//!
//! **Type**: helper
//!
//! Mark field as lazy.
//!
//! ### **`rename`**
//!
//! **Type**: function
//!
//! Specify alternative name for the field. The alternative will be used to form method names and, with `serde` feature
//! enabled, serialization name[^unless_in_serde].
//!
//! [^unless_in_serde]: Unless a different alternative name is specified for serialization with `serde` argument.
//!
//! ### **`get`**, **`get_mut`**, **`set`**, **`reader`**, **`writer`**, **`clearer`**, **`predicate`**, **`optional`**
//!
//! **Type**: helper
//!
//! Have similar syntax and semantics to corresponding `fxstruct` arguments:
//!
//! - [`get`](#get)
//! - [`get_mut`](#get_mut)
//! - [`set`](#set)
//! - [`reader` and `writer`](#reader-writer)
//! - [`clearer`](#clearer)
//! - [`predicate`](#predicate)
//! - [`optional`](#optional)
//!
//! ### **`optional`**
//!
//! **Type**: keyword
//!
//! Explicitly mark field as optional even if neither `predicate` nor `clearer` are requested.
//!
//! ### **`public(...)`**, **`private`**
//!
//! Field-default visibility for helper methods. See [the sub-arguments section](#sub_args) above for more details.
//!
//! ### **`serde`**
//!
//! **Type**: function
//!
//! At the field-level this option acts mostly the same way, as [at the struct-level](#serde). With a couple of
//! differences:
//!
//! - string literal sub-argument is bypassed into `serde` [field-level `rename`](https://serde.rs/field-attrs.html#rename)
//! - `default` is responsible for field default value; contrary to the struct-level, it doesn't use [`Into`] trait
//! - `attributes` will be applied to the field itself
//! - `serialize`/`deserialize` control field marshalling
//!
//! ### **`into`**
//!
//! **Type**: keyword
//!
//! Sets default for `set` and `builder` arguments.
//!
//! ### **`builder`**
//!
//! **Type**: function
//!
//! Mostly identical to the [struct-level `builder`](#builder). Field specifics are:
//!
//! - no `attributes_impl` (consumed, but ignored)
//! - string literal specifies setter method name if the builder type for this field
//! - `attributes` and `attributes_fn` are correspondingly applies to builder field and builder setter method
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
//! # The Inner Workings
//!
//! As it was mentioned in the [Basics](#basics) section, `fieldx` rewrites structures with `fxstruct` applied. The
//! following table reveals the final types of fields. `T` in the table represents the original field type, as specified
//! by the user; `O` is the original struct type.
//!
//! | Field Parameters | Non-Sync Type | Sync Type |
//! |------------------|---------------|-----------|
//! | `lazy` | `OnceCell<T>` | [`FXProxy<O, T>`] |
//! | optional (also activated with `clearer` and `proxy`) | `Option<T>` | [`FXRwLock<Option<T>>`] |
//! | `reader` and/or `writer` | N/A | [`FXRwLock<T>`] |
//!
//! Apparently, skipped fields retain their original type. Sure enough, if such a field is of non-`Send` or non-`Sync`
//! type the entire struct would be missing these traits despite all the efforts from the `fxstruct` macro.
//!
//! There is also a difference in how the initialization of `lazy` fields is implemented. Non-sync structs do it
//! directly in their accessor methods. Sync structs delegate this functionality to the [`FXProxy`] type.
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

pub mod errors;
pub mod traits;

pub use fieldx_derive::fxstruct;
#[doc(hidden)]
pub use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::{any, borrow::Borrow, cell::RefCell, fmt::Debug, marker::PhantomData, ops::Deref, sync::atomic::AtomicBool};
#[doc(hidden)]
pub use std::{cell::OnceCell, fmt, sync::atomic::Ordering};
use traits::FXStructSync;

/// Container type for lazy fields
///
/// Direct use of this struct is not recommended. See [reader and writer helpers](mod@crate#reader_writer_helpers).
pub struct FXProxy<O, T>
where
    O: FXStructSync,
{
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<fn(&O) -> T>>,
}

/// Lock-protected container
///
/// This is a wrapper around [`RwLock`] sync primitive. It provides safe means of cloning the lock and the data it
/// protects.
#[derive(Default)]
pub struct FXRwLock<T>(RwLock<T>);

/// Write-lock returned by [`FXProxy::write`] method
///
/// This type, in cooperation with the [`FXProxy`] type, takes care of safely updating lazy field status when data is
/// being stored.
#[allow(private_bounds)]
pub struct FXWrLock<'a, O, T>
where
    O: FXStructSync,
{
    lock:     RefCell<RwLockWriteGuard<'a, Option<T>>>,
    fxproxy:  &'a FXProxy<O, T>,
    _phantom: PhantomData<O>,
}

impl<O, T: fmt::Debug> fmt::Debug for FXProxy<O, T>
where
    O: FXStructSync,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vlock = self.value.read();
        formatter
            .debug_struct(any::type_name::<Self>())
            .field("value", &*vlock)
            .finish()
    }
}

impl<O, T> From<(fn(&O) -> T, Option<T>)> for FXProxy<O, T>
where
    O: FXStructSync,
{
    fn from((builder_method, value): (fn(&O) -> T, Option<T>)) -> Self {
        Self::new_default(builder_method, value)
    }
}

impl<O, T> FXProxy<O, T>
where
    O: FXStructSync,
{
    #[doc(hidden)]
    pub fn new_default(builder_method: fn(&O) -> T, value: Option<T>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }

    /// Consumes the container, returns the wrapped value or None if the container is empty
    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }

    #[inline]
    fn is_set_raw(&self) -> &AtomicBool {
        &self.is_set
    }

    /// Returns `true` if the container has a value.
    #[inline]
    pub fn is_set(&self) -> bool {
        self.is_set_raw().load(Ordering::SeqCst)
    }

    /// Initialize the field without obtaining the lock. Note though that if the lock is already owned this method will
    /// wait for it to be released.
    pub fn lazy_init<'a>(&'a self, owner: &O) {
        let _ = self.read_or_init(owner);
    }

    fn read_or_init<'a>(&'a self, owner: &O) -> RwLockUpgradableReadGuard<'a, Option<T>> {
        let guard = self.value.upgradable_read();
        if (*guard).is_none() {
            let mut wguard = RwLockUpgradableReadGuard::upgrade(guard);
            // Still uninitialized? Means no other thread took care of it yet.
            if wguard.is_none() {
                // No value has been set yet
                match *self.builder.read() {
                    Some(ref builder_cb) => {
                        *wguard = Some((*builder_cb)(owner));
                        self.is_set_raw().store(true, Ordering::SeqCst);
                    }
                    None => panic!("Builder is not set"),
                }
            }
            RwLockWriteGuard::downgrade_to_upgradable(wguard)
        }
        else {
            guard
        }
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct access to it without the [`Option`] wrapper.
    pub fn read<'a>(&'a self, owner: &O) -> MappedRwLockReadGuard<'a, T> {
        RwLockReadGuard::map(
            RwLockUpgradableReadGuard::downgrade(self.read_or_init(owner)),
            |data: &Option<T>| data.as_ref().unwrap(),
        )
    }

    /// Since the container guarantees that reading from it initializes the wrapped value, this method provides
    /// semit-direct mutable access to it without the [`Option`] wrapper.
    pub fn read_mut<'a>(&'a self, owner: &O) -> MappedRwLockWriteGuard<'a, T> {
        RwLockWriteGuard::map(
            RwLockUpgradableReadGuard::upgrade(self.read_or_init(owner)),
            |data: &mut Option<T>| data.as_mut().unwrap())
    }

    /// Provides write-lock to directly store the value.
    pub fn write<'a>(&'a self) -> FXWrLock<'a, O, T> {
        FXWrLock::<'a, O, T>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

    /// Resets the container into unitialized state
    pub fn clear(&self) -> Option<T> {
        let mut wguard = self.value.write();
        self.clear_with_lock(&mut wguard)
    }
}

#[allow(private_bounds)]
impl<'a, O, T> FXWrLock<'a, O, T>
where
    O: FXStructSync,
{
    #[doc(hidden)]
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FXProxy<O, T>) -> Self {
        let lock = RefCell::new(lock);
        Self {
            lock,
            fxproxy,
            _phantom: PhantomData,
        }
    }

    /// Store a new value into the container and returns the previous value or `None`.
    pub fn store(&mut self, value: T) -> Option<T> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

    /// Resets the container into unitialized state
    pub fn clear(&self) -> Option<T> {
        self.fxproxy.clear_with_lock(&mut *self.lock.borrow_mut())
    }
}

impl<O, T> Clone for FXProxy<O, T>
where
    O: FXStructSync,
    T: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.value.read();
        let bguard = self.builder.read();
        Self {
            value:   RwLock::new((*vguard).as_ref().cloned()),
            is_set:  AtomicBool::new(self.is_set()),
            builder: RwLock::new(bguard.clone()),
        }
    }
}

impl<T> FXRwLock<T> {
    #[doc(hidden)]
    pub fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    /// Consumes the lock and returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> From<T> for FXRwLock<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(value))
    }
}

impl<T> Deref for FXRwLock<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<RwLock<T>> for FXRwLock<T> {
    fn as_ref(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Borrow<RwLock<T>> for FXRwLock<T> {
    fn borrow(&self) -> &RwLock<T> {
        &self.0
    }
}

impl<T> Clone for FXRwLock<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let vguard = self.0.read();
        Self(RwLock::new((*vguard).clone()))
    }
}

impl<T> Debug for FXRwLock<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
