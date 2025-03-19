mod codegen;
mod ctx;
mod field_receiver;
mod helper;
mod input_receiver;
mod util;

// use std::panic::{catch_unwind, set_hook};
use crate::{input_receiver::FXInputReceiver, util::args::FXSArgs};
use darling::{ast, FromDeriveInput, FromMeta};
use syn::{parse_macro_input, DeriveInput};

// use rust_format::{Config, Edition, Formatter, RustFmt};
// #[allow(dead_code)]
// fn prettify_tok(item: TokenStream) -> String {
//     let cfg = Config::new_str().edition(Edition::Rust2021);
//     let rustfmt = RustFmt::from_config(cfg);
//     rustfmt.format_str(item.to_string()).unwrap()
// }

/// This macro implements `fieldx` functionality for a `struct` with named fields.
///
/// The macro provides two attributes: `#[fxstruct]` and `#[fieldx]`. The former is referred to as _"struct level"_
/// because it is the attribute applied to the struct itself. The latter defines individual field properties and,
/// correspondingly, is referred to as _"field level"_. Both attributes accept a number of arguments, most of which are
/// shared between two but may have somewhat different semantics. Also, arguments of `#[fxstruct]` are usually
/// responsible for specifying defaults for field level.
///
/// # Attribute Arguments
///
/// <a id="attr_terminology"></a>
/// ## Terminology
///
/// As it was mentioned above, the attributes accept arguments. I.e., for `#[fxstruct(sync, rc)]` words `sync` and `rc`
/// are the arguments. In cases when an argument semantics can be regulated with additional parameters these are called
/// _subarguments_ in this documentation. For example `off` in `#[fieldx(get(off))]` is a subargument of argument `get`.
/// A subargument in some cases may also take subarguments of its own.
///
/// Arguments and subarguments are classified depending on their syntax and functionality. Here is a list of their
/// types:
///
/// * _keyword_ – boolean-like, only accepts `off`: `keyword(off)`
/// * _flag_ – similar to the _keyword_ above but takes no arguments; as a matter of fact, the `off` above is a _flag_
/// * _helper_ - introduce functionality that is bound to a helper method (see below)
/// * _list_ or _function_ – can take multiple sub-arguments
/// * _meta_ - can take some syntax constructs
///
/// Field **Type**, that starts each argument description, specifies on of the above types.
///
/// ## Helper Methods
///
/// These implement certain functionality either provided by `fieldx` or required to be provided by user. For now the
/// only kind of required helpers are lazy builders. Others are generated by this this macro.
///
/// <a id="sub_args"></a>
/// ## Common Sub-Arguments of Helper Arguments
///
/// Helper arguments share many common sub-arguments. They are listed in the following table.  However, if their
/// meaning is unclear, it is recommended to skip this section and revisit it later.
///
/// | Sub-argument | Struct Level | Field Level |
/// |-|-|-|
/// | **`off`** | disable helper | disable helper |
/// | a non-empty string literal (**`"foo"`**) | method name prefix | explicit method name (struct level prefix is not used) |
/// | **`attributes_fn`** | default attributes for corresponding kind of helper methods | attributes for field's helper method |
/// | <a id="visibility"></a> **`vis`, `private`**; `private` is an alias for `vis()`| default visibility | visibility for field helper  |
///
/// For example:
///
/// ```ignore
/// #[fxstruct( get( "get_", vis(pub(crate)) ) )]
/// ```
///
/// will generate accessor methods with names prefixed with `get_` and visibility `pub(crate)`:
///
/// ```ignore
/// let foo = obj.get_foo();
/// ```
///
/// With:
///
/// ```ignore
/// #[fieldx( get( "special_type", private ) )]
/// ty: String,
/// ```
///
/// a method of the field owning struct can use the accessor as follows:
///
/// ```ignore
/// let foo = self.special_type();
/// ```
///
/// <a id="attrs_family"></a>
/// ## `attributes*` Family of Sub-Arguments
///
/// Sometimes it might be necessary to specify additional attributes for various generated syntax elements like methods,
/// or auxiliary structs. Where applicable, this functionality is supported by `attributes*` (sub)arguments. Their
/// syntax is `attributes(<attr1>, <attr2>, ...)` where an `<attr>` looks exactly, as it would in
/// the code but excluding the `#[...]` wrapping.
///
/// For example, `attributes_fn(allow(dead_code), cfg(feature = "myfeature"))` will expand into something like:
///
/// ```ignore
/// #[allow(dead_code)]
/// #[cfg(feature = "myfeature")]
/// ```
///
/// The following members of the family are currently supported: `attributes`, `attributes_fn`, and `attributes_impl`.
/// Which ones are implemented for a particular context is documented below.
///
/// ## Struct Level Arguments
///
/// ### **`attributes`**
///
/// **Type**: `list`
///
/// Fallback [attributes](#attrs_family) for structs produced by the `builder` and `serde` arguments. I.e. when
/// [`builder`](#builder_struct) or [`serde`](#serde_struct) are requested but don't have their own `attributes`
/// then this one will be used.
///
/// ### **`attributes_impl`**
///
/// **Type**: `list`
///
/// [Attributes](#attrs_family) to be applied to the struct implementation.
///
/// ### **`sync`**
///
/// **Type**: keyword
///
/// Declare a struct as thread-safe by default.
///
/// ### **`r#async`***
///
/// **Type**: keyword
///
/// Declare a struct as async by default.
///
/// *Note:* Since `async` is a keyword, the `syn` is not allowing to use it as-is, only with the `r#` prefix, according
/// to Rust syntax.
///
/// ### **`mode`**
///
/// **Type**: function
///
/// This is another way to specify the default concurrency mode for struct. It takes one of three keywords as arguments:
///
/// - `sync`
/// - `async`
/// - `plain`
///
/// Note that contrary to the direct keyword way, `async` doesn't require the `r#` prefix: `mode(async)`.
///
/// Also, there is no `plain` keyword, but one can use it with `mode` as an explicit marker.
///
/// ### **`lazy`**
///
/// **Type**: helper
///
/// Enables lazy mode for all fields except those marked with `lazy(off)`.
///
/// ### **`fallible`**
///
/// **Type**: function
///
/// Enable fallible lazy builders, i.e. expects them to return [`Result`] enum. Takes two sub-arguments:
///
/// - `off` - disables fallible functionality
/// - `error(ErrorType)` – set the expected error, returned by builder.
///
/// `fallible(off, error(MyError))` simply sets default error type for all fallible lazy fields.
///
/// ### ***inner_mut***
///
/// **Type**: keyword
///
/// Turns on interior mutability for struct fields by default.
///
/// <a id="builder_struct"></a>
/// ### **`builder`**
///
/// **Type**: helper
///
/// Enables builder functionality by introducing a `builder()` associated function and builder type:
///
/// ```
/// # use fieldx::fxstruct;
/// #[fxstruct(builder, get)]
/// struct Foo {
///     description: String,
/// }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let obj = Foo::builder()
///                .description(String::from("some description"))
///                .build()?;
/// assert_eq!(obj.description(), "some description");
/// # Ok(())
/// # }
/// ```
///
/// Literal string sub-argument of `builder` defines common prefix for methods-setters of the builder. For example, with
/// `builder("set_")` one would then use `.set_description(...)` call.
///
/// Additional sub-arguments:
///
/// - **`attributes`** (see the [section above](#attrs_family)) – builder struct attributes
/// - **`attributes_impl`** - attributes of the struct implementation
/// - **`into`** – force all builder setter methods to attempt automatic type conversion using `.into()` method
///
///   With `into` the example above wouldn't need `String::from` and the call could look like this:
///   `.description("some description")`
/// - **`opt_in`** - struct-level only argument; with it only fields with explicit `builder` can be set by builder.
/// - **`init`** - struct-level only argument; specifies identifier of the method to call to finish object initialization.
/// - **`post_build`** - struct-level only; makes builder's `build()` method to call `post_build()` method. If given an
///   ident argument it specifies different method name: `post_build(check_in)`.
///
///   There are a couple of notes to take into account:
///
///   * the method is called on freshly created object right before it is returned back to builder caller
///   * it must take and return `self`: `fn post_build(mut self) { self.foo = "bar"; self }`
///   * for reference-counted structs the method is invoked before they're wrapped into corresponding container;
///     this allows for `mut self` and direct access to the fields without use of inner mutability
/// - **`error(ErrorType)`** - struct-level only; changes the error type returned by the `build()` method. Together with
///   the `post_build` argument makes the post-build method fallible with the same error type.
///
///   **Note** that the builder code is always producing `FieldXError::UninitializedField` variant. Therefore, to be
///   compatible with it the custom `ErrorType` must implement `From<FieldXError>`.
///
/// ### **`rc`**
///
/// **Type**: keyword
///
/// With this argument new instances of the type, produced by the `new` method or by type's builder, will be wrapped
/// into reference counting pointers `Rc` or `Arc`, depending on `sync` status of the type.
///
/// ### **`no_new`**
///
/// **Type**: keyword
///
/// Disable generation of method `new`. This is useful for cases when a user wants their own `new` method.
///
/// With this option the macro may avoid generating `Default` implementation for the struct. More details in [a section
/// below](#about_default).
///
/// ### **`default`**
///
/// **Type**: keyword
///
/// Forces the `Default` implementation to be generated for the struct.
///
/// ### **`get`**
///
/// **Type**: helper
///
/// Enables or disables getter methods for all fields, unless a field is marked otherwise.
///
/// Additionally to the standard helper arguments accessors can also be configured as:
///
/// - **`clone`** - cloning, i.e. returning a clone of the field value (must implement [`Clone`])
/// - **`copy`** - copying, i.e. returning a copy of the field value (must implement [`Copy`])
/// - **`as_ref`** – only applicable if field value is optional; it makes the accessor to return an `Option<&T>`
///   instead of `&Option<T>`
///
/// ### **`get_mut`**
///
/// **Type**: helper
///
/// Request for a mutable accessor. Since neither of additional options of `get` are applicable here[^no_copy_for_mut]
/// only basic [helper sub-arguments](#sub_args) are accepted.
///
/// Mutable accessors have the same name, as immutable ones, but with `_mut` suffix, unless given explicit name by the
/// user:
///
/// ```
/// # use fieldx::fxstruct;
/// #[fxstruct(get, get_mut)]
/// struct Foo {
///     description: String,
/// }
/// # fn main() {
/// let mut obj = Foo::new();
/// *obj.description_mut() = "some description".to_string();
/// assert_eq!(obj.description(), "some description");
/// # }
/// ```
///
/// [^no_copy_for_mut]: What sense is in having a mutable copy if you own it already?
///
/// ### **`set`**
///
/// **Type**: helper
///
/// Request for setter methods. If a literal string sub-argument is supplied it is used as setter method prefix instead
/// of the default `set_`.
///
/// Takes an additional sub-argument:
///
/// - **`into`**: use the [`Into`] trait to automatically convert a value into the field type
///
/// ```
/// # use fieldx::fxstruct;
/// #[fxstruct(set(into), get)]
/// struct Foo {
///     description: String,
/// }
/// # fn main() {
/// let mut obj = Foo::new();
/// obj.set_description("some description");
/// assert_eq!(obj.description(), &"some description".to_string());
/// # }
/// ```
///
/// <a id="reader_writer_helpers"></a>
/// ### **`reader`**, **`writer`**
///
/// **Type**: helper
///
/// Only meaningful for `sync` structs. Request for reader and writer methods that would return either read-only or
/// read-write lock guards.
///
/// Akin to setters, method names are formed using `read_` and `write_` prefixes, correspondingly, prepended to the
/// field name.
///
/// ```
/// # use fieldx::fxstruct;
/// # fn main() {
/// #[fxstruct(sync, reader, writer)]
/// struct Foo {
///     description: String,
/// }
/// let obj = Foo::new();
/// {
///     let mut wguard = obj.write_description();
///     *wguard = String::from("let's use something different");
/// }
/// {
///     let rguard = obj.read_description();
///     assert_eq!(*rguard, "let's use something different".to_string());
/// }
/// # }
/// ```
///
/// See [the section about differences between `get`/`get_mut` and `reader`/`writer`](#accessor_vs_reader_writer)
///
/// ### **`lock`**
///
/// **Type**: flag
///
/// Forces lock-wrapping of all fields by default. Can be explicitly disabled with `lock(off)`. Identical to the
/// `reader`/`writer` arguments but without installing any methods.
///
/// ### **`clearer`** and **`predicate`**
///
/// **Type**: helper
///
/// These two are tightly coupled by their meaning, though can be used separately.
///
/// Predicate helper methods return [`bool`] and are the way to find out if a field is set. They're universal in the way
/// that no matter wether a field is sync, or plain, or lazy, or just optional – you always use the same method.
///
/// Clearer helpers are the way to reset a field into uninitialized state. For optional fields it would simply mean it
/// will contain [`None`]. A lazy field would be re-initialized the next time it is read from.
///
/// Clearers return the current field value. If field is already uninitialized (or never has been yet) `None` will be
/// given back.
///
/// Using either of the two automatically make fields optional unless lazy.
///
/// Check out the [example](#optional_example) in the [Optional Fields](#optional_fields) section.
///
/// ### **`optional`**
///
/// **Type**: keyword
///
/// Explicitly make all fields optional. Useful when neither predicate nor clearer helpers are needed.
///
/// ### **`vis(...)`**, **`private`**
///
/// Specify defaults for helpers. See [the sub-arguments section](#sub_args) above for more details.
///
/// ### **`clone`**, **`copy`**
///
/// Specify defaults for accessor helpers.
///
/// <a id="serde_struct"></a>
/// ### **`serde`**
///
/// **Type**: [function](#attr_terminology)
///
/// Enabled with `serde` feature, which is off by default.
///
/// Support for de/serialization will be discussed in more details in a section below. What is important to know at this
/// point is that due to use of container types direct serialization of a struct is hardly possible. Therefore `fieldx`
/// utilizes `serde`'s `into` and `from` by creating a special shadow struct. The shadow, by default, is named after the
/// original by prepending the name with double underscore and appending *Shadow* suffix: `__FooShadow`.
///
/// The following sub-arguments are supported:
///
/// - a string literal is used to as alternative name for serialization (see `rename` below)
/// - **`off`** disables de/serialization support altogether
/// - **`attributes(...)`** - custom [attributes](#attrs_family) to be applied to the shadow struct
/// - **`vis(...)`**, **`private`** – specify [visibility](#visibility) of the shadow struct
/// - **`serialize`** - enable or disable (`serialize(off)`) serialization support for the struct
/// - **`deserialize`** - enable or disable (`deserialize(off)`) deserialization support for the struct
/// - **`default`** - whether `serde` must use defaults for missing fields and, perhaps, where to take the defaults from
/// - **`forward_attrs`** - a list of field attributes that are to be forwarded to the corresponding field of the shadow
///   struct
/// - **`rename(serialize(...), deserialize(...))`** - defines values for `serde`
///   [`rename`](https://serde.rs/container-attrs.html#rename). Can also be used with a single string literal which
///   would then set both `serialize` and `deserialize` at once.
/// - **`shadow_name(...)`** - its string literal argument specifies a different name for the shadow struct
///
/// ##### _Notes about `default`_
///
/// Valid arguments for the sub-argument are:
///
/// * a string literal that has the same meaning as for
///   [the container-level `serde` attribute `default`](https://serde.rs/container-attrs.html#default--path)
/// * a path to a symbol that is bound to an instance of our type: `my_crate::FOO_DEFAULT`
/// * a call-like path that'd be used literally: `Self::serde_default()`
///
/// The last option is preferable because `fieldx` will parse it and replace any found `Self` reference with the
/// actual structure name making possible future renaming of it much easier.
///
/// There is a potentially useful "trick" in how `default` works. Internally, whatever type is returned by the
/// sub-argument it gets converted into the shadow type with trait [`Into`]. This allows you to use the original struct
/// as the trait implementation is automatically generated for it. See this example from a test:
///
/// ```
/// # use fieldx::fxstruct;
/// # use serde::{Serialize, Deserialize};
/// #[fxstruct(sync, get, serde(shadow_name("BazDup"), default(Self::serde_default())))]
/// #[derive(Clone)]
/// struct Baz {
///     #[fieldx(reader)]
///     f1: String,
///     f2: String,
/// }
///
/// impl Baz {
///     fn serde_default() -> Fubar {
///         Fubar {
///             postfix: "from fubar".into()
///         }
///     }
/// }
///
/// struct Fubar {
///     postfix: String,
/// }
///
/// impl From<Fubar> for BazDup {
///     fn from(value: Fubar) -> Self {
///         Self {
///             f1: format!("f1 {}", value.postfix),
///             f2: format!("f2 {}", value.postfix),
///         }
///     }
/// }
///
/// # fn main() {
/// let json_src = r#"{"f1": "f1 json"}"#;
/// let foo_de = serde_json::from_str::<Baz>(&json_src).expect("Bar deserialization failure");
/// assert_eq!(*foo_de.f1(), "f1 json".to_string());
/// assert_eq!(*foo_de.f2(), "f2 from fubar".to_string());
/// # }
/// ```
///
/// ## Field Level Arguments
///
/// At this point, it's worth refreshing your memory about [sub-arguments of helpers](#sub_args) and how they differ in
/// semantics between `fxstruct` and `fieldx` attributes.
///
/// ### **`skip`**
///
/// **Type**: flag
///
/// Leave this field alone. The only respected argument of `fieldx` when skipped is the `default`.
///
/// ### **`lazy`**
///
/// **Type**: helper
///
/// Mark field as lazy.
///
/// ### **`fallible`**
///
/// Lets lazy builder method to return an error. For example:
///
/// ```ignore
/// #[fxstruct()]
/// struct Foo {
///     #[fieldx(lazy, fallible(error(MyError)))]
///     connection: Resource,
/// }
///
/// impl Foo {
///     fn build_connection(&self) -> Result<Resource, MyError> {
///         Err(MyError::AdHoc)
///     }
/// }
///
/// let foo = Foo.new();
/// assert!(foo.connection().is_err());
/// ```
///
/// Now, field accessor would be returning `Result<T, MyError>`, where the exact type `T` depends on other field
/// paramters.
///
/// If many struct fields need to be fallible while common error type is used for each one it would make sense to
/// declare the type once for the entire struct:
///
/// ```ignore
/// #[fxstruct( fallible(off, error(MyError)) )]
/// struct Foo {
///     #[fieldx(lazy, fallible)]
///     connection: Resource,
///     #[fieldx(lazy, fallible)]
///     resource_manager: ResourceManager,
/// }
/// ```
///
/// ### **inner_mut***
///
/// **Type**: keyword
///
/// Enables field interior mutability.
///
/// ### **`rename`**
///
/// **Type**: function
///
/// Specify alternative name for the field. The alternative will be used to form method names and, with `serde` feature
/// enabled, serialization name[^unless_in_serde].
///
/// [^unless_in_serde]: Unless a different alternative name is specified for serialization with `serde` argument.
///
/// ### **`get`**, **`get_mut`**, **`set`**, **`reader`**, **`writer`**, **`clearer`**, **`predicate`**, **`optional`**
///
/// **Type**: helper
///
/// Have similar syntax and semantics to corresponding `fxstruct` arguments:
///
/// - [`get`](#get)
/// - [`get_mut`](#get_mut)
/// - [`set`](#set)
/// - [`reader` and `writer`](#reader-writer)
/// - [`clearer`](#clearer)
/// - [`predicate`](#predicate)
/// - [`optional`](#optional)
///
/// ### **`optional`**
///
/// **Type**: keyword
///
/// Explicitly mark field as optional even if neither `predicate` nor `clearer` are requested.
///
/// ### **`vis(...)`**, **`private`**
///
/// Field-default visibility for helper methods. See [the sub-arguments section](#sub_args) above.
///
/// ### **`serde`**
///
/// **Type**: function
///
/// At the field-level this option acts mostly the same way, as [at the struct-level](#serde). With a couple of
/// differences:
///
/// - string literal sub-argument is bypassed into `serde` [field-level `rename`](https://serde.rs/field-attrs.html#rename)
/// - `default` is responsible for field default value; contrary to the struct-level, it doesn't use [`Into`] trait
/// - `attributes` will be applied to the field itself
/// - `serialize`/`deserialize` control field marshalling
///
/// ### **`into`**
///
/// **Type**: keyword
///
/// Sets default for `set` and `builder` arguments.
///
/// ### **`builder`**
///
/// **Type**: helper
///
/// Mostly identical to the [struct-level `builder`](#builder). Field specifics are:
///
/// - no `attributes_impl` and `opt_in` (consumed, but ignored)
/// - string literal specifies setter method name of the builder type for this field
/// - `attributes` and `attributes_fn` are correspondingly applies to builder field and builder setter method
///
/// Field level only argument:
///
/// - **`required`** – this field must always get a value from the builder even if otherwise it'd be optional
#[proc_macro_attribute]
pub fn fxstruct(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr_args = match ast::NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return darling::Error::from(e).write_errors().into();
        }
    };

    let args = match FXSArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    let input_ast = parse_macro_input!(input as DeriveInput);
    let fx = match FXInputReceiver::from_derive_input(&input_ast) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    codegen::FXRewriter::new(fx, args).rewrite().into()
}
