[![Rust](https://github.com/vrurg/fieldx/actions/workflows/fieldx.yml/badge.svg)](https://github.com/vrurg/fieldx/actions/workflows/fieldx.yml)
![License](https://img.shields.io/github/license/vrurg/fieldx)
![Crates.io Version](https://img.shields.io/crates/v/fieldx)

# fieldx v0.1.8-beta.2

Procedural macro for constructing structs with lazily initialized fields, builder pattern, and [`serde`] support
with a focus on declarative syntax.

Let's start with an example:

```rust
use fieldx::fxstruct;

#[fxstruct( lazy )]
struct Foo {
    count: usize,
    foo:   String,
    #[fieldx( lazy(off), get )]
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

- a struct with all fields been `lazy` by default
- laziness is explicitly disabled for field `order`
- methods `build_count` and `build_foo` return initial values for corresponding fields

At run-time we first ensure that the `order` vector is empty meaning none of the `build_` methods was called. Then
we read from `foo` using its accessor method. Then we make sure that each `build_` method was invoked only once.

As one can notice, a minimal amount of handcraft is needed here as most of boilerplate is handled by the macro,
which provides even basic `new` associated function.

Also notice that we don't need to remember the order of initialization of fields. Builder of `foo` is using `count`
without worrying if it's been initialized yet or not because it will always be.

## Basics

The module provides two attributes: `fxstruct`, and `fieldx`. The first is responsible for configuring structs, the
second for adjusting field parameters.

The macro can only be used with named structures, no union types, nor enums are supported. When applied, it rewrites
the type it is applied to according to the parameters provided. Here is a list of most notable changes and
additions:

- field types may be be wrapped into container types

  In the above example `foo` and `count` become [`OnceCell<String>`][OnceCell] and `OnceCell<usize>`, whereas
  `order` remains unchanged.

- a partial implementation of `Foo` is added with support methods and associated functions

  I.e. this is where accessor methods and `new` live.

- depending on parameters, an implicit implementation of the [`Default`] trait may be be added
- if requested, builder struct and `builder()` associated function will be implemented
- also, if requested, a shadow struct for correct `serde` support will be there too

**Note** that user is highly discouraged from directly accessing modified fields. The module does its best to
provide all necessary API via corresponding methods.

## Sync, Async, And Plain Structs

_Note:_ "Async" is considered to be a synonym to "sync" since both require concurrency safety. Even the code
generated for sync and async cases is mostly identical.

If a thread-safe struct is needed then `fxstruct` must take the `sync` argument: `#[fxstruct(sync, ...)]`. When
instructed so, the macro will do its best to provide concurrency safety at the field level. It means that:

- lazy builder methods are guaranteed to be invoked once and only once per each initialization, be it single- or
  multi-threaded application
- access to field is lock-protected for lazy or optional fields implicitly

In less strict cases it is possible to mark individual fields as sync.

Plain non-mutable accessors normally return a reference to their field. Accessors of sync structs, unless directed
to use [`clone`][`Clone`] or [`copy`][`Copy`], or used with a non-protected field, return some kind of lock-guard
object.

Wrapper types for sync struct fields are non-`std` and provided with the module.

<a id="protected_unprotected_fields"></a>
### Protected And Unprotected Fields Of Sync Structs

For a `fieldx` sync struct to be `Sync+Sent` all of its fields are expected to be _lock-protected_ (or, sometimes we
could just say _"protected"_). But "expected" doesn't mean "has to be". Unless defaults, specified with `fxstruct`
attribute (i.e. with _struct-level_ arguments) tell otherwise, fields not marked with `fieldx` attribute with
corresponding arguments will remain _unprotected_. I.e.:

```rust
#[fxstruct(sync)]
struct Foo {
    #[fieldx(lazy)]
    foo: String, // protected
    #[fieldx(get_mut)]
    bar: String, // unprotected
}
```

Of course, whether the struct remains thread-safe would then depend on the safety of unprotected fields.

<a id="optional_fields"></a>
## Optional Fields

_Optional_ in this context has the same meaning, as in the [`Option`] type. Sure thing, one can simply declare a
field using the core type (and, as a matter of fact, this is what `fieldx` is using internally anyway). What's the
advantages of using `fieldx` then?

First of all, manual declaration may mean additional boilerplate code to implement an accessor, among other things.
With `fieldx` most of it can be hidden under a single declaration:

<a id="optional_example"></a>
```rust
#[fxstruct]
struct Foo {
    #[fieldx(predicate, clearer, get, set(into))]
    description: String,
}

let mut obj = Foo::new();
assert!( !obj.has_description() );
obj.set_description("foo");
assert!( obj.has_description() );
assert_eq!( obj.description(), &Some(String::from("foo")) );
obj.clear_description();
assert!( !obj.has_description() );
```

_`<digression_mode>`_ Besides, aesthetically, to some `has_description` is more appealing than
`obj.description().is_some()`. _`</digression_mode>`_

Next, optional fields of `sync` structs are lock-protected by default. This can be changed with explicit
`lock(off)`, but one has to be aware that then sync status of the struct will depend the safety of the field.

And the last note to be made is that if at some point it would prove to be useful to convert a field into a `lazy`
then refactoring could be reduced to simply adding corresponding argument the `fieldx` attribute and implementing a
new builder for it.

## Laziness Protocol

Though being very simple concept, laziness has its own peculiarities. The basics, as shown above, are such that when
we declare a field as `lazy` the macro wraps it into some kind of proxy container type ([`OnceCell`] for plain
fields). The first read[^only_via_method] from an uninitialized field will result in the lazy builder method to be
invoked and the value it returns to be stored in the field.

Here come the caveats:

1. A builder is expected to be infallible. This requirement comes from the fact that when we call field's accessor
   we expect a value of field's type to be returned. Since Rust requires errors to be handled semi-in-place (contrary
   to exceptions in many other languages) there is no way for us to overcome this limitation. The builder could panic,
   but this is rarely a good option.

   For cases when it is important to have controllable error handling, one could give the field a [`Result`] type.
   Then `obj.field()?` could be a way to take care of errors. But this approach has its own complications,
   especially for sync fields.

1. Field builder methods cannot mutate their objects. This limitation also comes from the fact that a typical
   accessor method doesn't need and must not use mutable `&self`. Of course, it is always possible to use internal
   mutability, as in the first example here.

[^only_via_method]: Apparently, the access has to be made by calling a corresponding method. Mostly it'd be field's
accessor, but for `sync` structs it's more likely to be a reader.

## Field Interior Mutability

Marking fields with `inner_mut` flag is a shortcut for using `RefCell` wrapper. This effectively turns such fields
to be plain ones.

```rust
#[fxstruct]
struct Foo {
    #[fieldx(inner_mut, get, get_mut, set, default(String::from("initial")))]
    modifiable: String,
}

let foo = Foo::new();
let old = foo.set_modifiable(String::from("manual"));
assert_eq!(old, String::from("initial"));
assert_eq!(*foo.modifiable(), String::from("manual"));
*foo.modifiable_mut() = String::from("via mutable accessor");
assert_eq!(*foo.modifiable(), String::from("via mutable accessor"));
```

Note that this pattern is only useful when the field must not be neither optional nor lock-protected in
`sync`-declared structs.

## Builder Pattern

**IMPORTANT!** First of all, it is necessary to mention unintended terminological ambiguity here. The terms `build`
and `builder` are used for different, though identical in nature, processes. As mentioned in the previous section,
the _lazy builders_ are methods that return initial values for associated fields. The _struct builder_ in this
section is an object that collects initial values from user and then is able to create the final instance of the
original struct.  This ambiguity has some history spanning back to the times when Perl's
[`Moo`](https://metacpan.org/pod/Moo) module was one of the author's primary tools. Then it was borrowed by Raku
[`AttrX::Mooish`](https://raku.land/zef:vrurg/AttrX::Mooish) and, finally, automatically made its way into `fieldx`
which, initially, didn't implement the builder pattern.

The default `new` method generated by `fxstruct` macro accepts no arguments and simply creates a bare-bones object
initialized from type defaults. Submitting custom values for struct fields is better be done by using the
builder pattern:

```rust
#[fxstruct(builder)]
struct Foo {
    #[fieldx(lazy)]
    description: String,
    count: usize,
}

impl Foo {
    fn build_description(&self) -> String {
        format!("this is item #{}", self.count)
    }
}

let obj = Foo::builder()
            .count(42)
            .build()
            .expect("Foo builder failure");
assert_eq!( obj.description(), &String::from("this is item #42") );

let obj = Foo::builder()
            .count(13)
            .description(String::from("count is ignored"))
            .build()
            .expect("Foo builder failure");
// Since the `description` is given a value the `count` field is not used
assert_eq!( obj.description(), &String::from("count is ignored") );
```

Since the only `fieldx`-related failure that may happen when building a new object instance is a required field not
given a value, the `build()` method would return [`FieldXError`](error::FieldXError) if this happens.

## Crate Features

The following featues are supported by this crate:

| *Feature* | *Description* |
|-|-|
| `diagnostics` | Enable additional diagnostics for compile time errors. Requires Rust nightly toolset. |
| `serde` | Enable support for `serde` marshalling. |
| `send_guard` | See corresponding feature of the [`parking_lot` crate](https://crates.io/crates/parking_lot) |

## Usage

Most arguments of both `fxstruct` and `fieldx` can take either of the two forms: a keyword (`arg`), or a
*"function"* (`arg(subarg)`).

Also, most of the arguments are shared by both `fxstruct` and `fieldx`. But their meaning and the way their
arguments are interpreted could be slightly different for each attribute. For example, if an argument takes a
literal string sub-argument it is likely to be a method name when associated with `fieldx`; but for `fxstruct` it
would define common prefix for method names.

There is also a commonality between most of the arguments: they can be temporarily (say, for testing purposes) or
permanently turned off by using `off` sub-argument with them. See `lazy(off)` in the
above example.

## Attribute Arguments

<a id="attr_terminology"></a>
A few words on terminology:

- argument **Type** determines what sub-arguments can be received:
  * _keyword_ – boolean-like, only accepts `off`: `keyword(off)`
  * _flag_ – similar to the _keyword_ above but takes no arguments; as a matter of fact, the `off` above is a _flag_
  * _helper_ - introduce functionality that is bound to a helper method (see below)
  * _list_ or _function_ – can take multiple sub-arguments
  * _meta_ - can take some syntax constructs
- helper method – implements certain functionality

  Almost all helpers are generated by the macro. The only exception are lazy builders which must be provided by the
  user.
- **For** specifies if argument is specific to an attribute

<a id="sub_args"></a>
### Sub-Arguments of Helper Arguments

Helper arguments share a bunch of common sub-arguments. We will describe them here, but if their meaning is unclear
it'd be better to skip this section and get back to it later.

| Sub-argument | In fxstruct | In fxfield |
|-|-|-|
| **`off`** | disable helper | disable helper |
| a non-empty string literal (**`"foo"`**) | method name prefix | explicit method name (prefix not used) |
| **`attributes_fn`** | default attributes for corresponding kind of helper methods | attributes for field's helper method |
| <a id="visibility"></a> **`public`, `public(crate)`, `public(super)`, `public(some::module)`, `private`** | default visibility | visibility for field helper |

For example:

```rust
#[fxstruct( get( "get_", public(crate) ) )]
```

will generate accessor methods with names prefixed with `get_` and visibility `pub(crate)`:

```rust
let foo = obj.get_foo();
```

With:

```rust
#[fieldx( get( "special_type", private ) )]
ty: String,
```

a method of the field owning struct can use the accessor as follows:

```rust
let foo = self.special_type();
```

<a id="attrs_family"></a>
### `attributes*` Family of Sub-Arguments

Sometimes it might be necessary to specify attributes for various generated syntax elements like methods, or
auxiliary structs. Where applicable, this functionality is supported by `attributes*` (sub)arguments. Their syntax
is `attributes(<attr1>, <attr2>, ...)` where an `<attr>` is specified exactly, as it would be specified in the code,
but with starting `#[` and finishing `]` being omitted.

For example, `attributes_fn(allow(dead_code), cfg(feature = "myfeature"))` will expand into something like:

```rust
#[allow(dead_code)]
#[cfg(feature = "myfeature")]
```

The following members of the family are currently supported: `attributes`, `attributes_fn`, and `attributes_impl`.
Which ones are supported in a particular context is documented below.

### Arguments of `fxstruct`

#### **`attributes`**

**Type**: `list`

Fallback [attributes](#attrs_family) for structs produced by the `builder` and `serde` arguments. I.e. when
[`builder`](#builder_struct) or [`serde`](#serde_struct) are requested but don't have their own `attributes`
then this one will be used.

#### **`attributes_impl`**

**Type**: `list`

[Attributes](#attrs_family) to be applied to the struct implementation.

#### **`sync`**

**Type**: keyword

Declare a struct as thread-safe by default.

#### **`r#async`***

**Type**: keyword

Declare a struct as async by default.

*Note:* Since `async` is a keyword, the `syn` is not allowing to use it as-is, only with the `r#` prefix, according
to Rust syntax.

#### **`mode`**

**Type**: function

This is another way to specify the default concurrency mode for struct. It takes one of three keywords as arguments:

- `sync`
- `async`
- `plain`

Note that contrary to the direct keyword way, `async` doesn't require the `r#` prefix: `mode(async)`.

Also, there is no `plain` keyword, but one can use it with `mode` as an explicit marker.

#### **`lazy`**

**Type**: helper

Enables lazy mode for all fields except those marked with `lazy(off)`.

#### ***inner_mut***

**Type**: keyword

Turns on interior mutability for struct fields by default.

<a id="builder_struct"></a>
#### **`builder`**

**Type**: helper

Enables builder functionality by introducing a `builder()` associated function and builder type:

```rust
#[fxstruct(builder, get)]
struct Foo {
    description: String,
}
let obj = Foo::builder()
               .description(String::from("some description"))
               .build()?;
assert_eq!(obj.description(), "some description");
```

Literal string sub-argument of `builder` defines common prefix for methods-setters of the builder. For example, with
`builder("set_")` one would then use `.set_description(...)` call.

Additional sub-arguments:

- **`attributes`** (see the [section above](#attrs_family)) – builder struct attributes
- **`attributes_impl`** - attributes of the struct implementation
- **`into`** – force all builder setter methods to attempt automatic type conversion using `.into()` method

  With `into` the example above wouldn't need `String::from` and the call could look like this:
  `.description("some description")`
- **`opt_in`** - struct-level only argument; with it only fields with explicit `builder` can get their values from the builder
- **`init`** - struct-level only argument; specifies identifier of the method to call to finish object initialization.

  There are a couple of notes to take into account:

  - the method is called on freshly created object right before it is returned back to builder caller
  - it must take and return `self`: `fn post_build(mut self) { self.foo = "bar"; self }`
  - for reference-counted structs the method is invoked before they're wrapped into corresponding container;
    this allows for `mut self` and direct access to the fields without use of inner mutability

#### **`rc`**

**Type**: keyword

With this argument new instances of the type, produced by the `new` method or by type's builder, will be wrapped
into reference counting pointers `Rc` or `Arc`, depending on `sync` status of the type.

#### **`no_new`**

**Type**: keyword

Disable generation of method `new`. This is useful for cases when a user wants their own `new` method.

With this option the macro may avoid generating `Default` implementation for the struct. More details in [a section
below](#about_default).

#### **`default`**

**Type**: keyword

Forces the `Default` implementation to be generated for the struct.

#### **`get`**

**Type**: helper

Enables or disables getter methods for all fields, unless a field is marked otherwise.

Additionally to the standard helper arguments accessors can also be configured as:

- **`clone`** - cloning, i.e. returning a clone of the field value (must implement [`Clone`])
- **`copy`** - copying, i.e. returning a copy of the field value (must implement [`Copy`])
- **`as_ref`** – only applicable if field value is optional; it makes the accessor to return an `Option<&T>`
  instead of `&Option<T>`

#### **`get_mut`**

**Type**: helper

Request for a mutable accessor. Since neither of additional options of `get` are applicable here[^no_copy_for_mut]
only basic [helper sub-arguments](#sub_args) are accepted.

Mutable accessors have the same name, as immutable ones, but with `_mut` suffix, unless given explicit name by the
user:

```rust
#[fxstruct(get, get_mut)]
struct Foo {
    description: String,
}
let mut obj = Foo::new();
*obj.description_mut() = "some description".to_string();
assert_eq!(obj.description(), "some description");
```

[^no_copy_for_mut]: What sense is in having a mutable copy if you own it already?

#### **`set`**

**Type**: helper

Request for setter methods. If a literal string sub-argument is supplied it is used as setter method prefix instead
of the default `set_`.

Takes an additional sub-argument:

- **`into`**: use the [`Into`] trait to automatically convert a value into the field type

```rust
#[fxstruct(set(into), get)]
struct Foo {
    description: String,
}
let mut obj = Foo::new();
obj.set_description("some description");
assert_eq!(obj.description(), &"some description".to_string());
```

<a id="reader_writer_helpers"></a>
#### **`reader`**, **`writer`**

**Type**: helper

Only meaningful for `sync` structs. Request for reader and writer methods that would return either read-only or
read-write lock guards.

Akin to setters, method names are formed using `read_` and `write_` prefixes, correspondingly, prepended to the
field name.

```rust
#[fxstruct(sync, reader, writer)]
struct Foo {
    description: String,
}
let obj = Foo::new();
{
    let mut wguard = obj.write_description();
    *wguard = String::from("let's use something different");
}
{
    let rguard = obj.read_description();
    assert_eq!(*rguard, "let's use something different".to_string());
}
```

See [the section about differences between `get`/`get_mut` and `reader`/`writer`](#accessor_vs_reader_writer)

#### **`lock`**

**Type**: flag

Forces lock-wrapping of all fields by default. Can be explicitly disabled with `lock(off)`. Identical to the
`reader`/`writer` arguments but without installing any methods.

#### **`clearer`** and **`predicate`**

**Type**: helper

These two are tightly coupled by their meaning, though can be used separately.

Predicate helper methods return [`bool`] and are the way to find out if a field is set. They're universal in the way
that no matter wether a field is sync, or plain, or lazy, or just optional – you always use the same method.

Clearer helpers are the way to reset a field into uninitialized state. For optional fields it would simply mean it
will contain [`None`]. A lazy field would be re-initialized the next time it is read from.

Clearers return the current field value. If field is already uninitialized (or never has been yet) `None` will be
given back.

Using either of the two automatically make fields optional unless lazy.

Check out the [example](#optional_example) in the [Optional Fields](#optional_fields) section.

#### **`optional`**

**Type**: keyword

Explicitly make all fields optional. Useful when neither predicate nor clearer helpers are needed.

#### **`public(...)`**, **`private`**

Specify defaults for helpers. See [the sub-arguments section](#sub_args) above for more details.

#### **`clone`**, **`copy`**

Specify defaults for accessor helpers.

<a id="serde_struct"></a>
#### **`serde`**

**Type**: [function](#attr_terminology)

Enabled with `serde` feature, which is off by default.

Support for de/serialization will be discussed in more details in a section below. What is important to know at this
point is that due to use of container types direct serialization of a struct is hardly possible. Therefore `fieldx`
utilizes `serde`'s `into` and `from` by creating a special shadow struct. The shadow, by default, is named after the
original by prepending the name with double underscore and appending *Shadow* suffix: `__FooShadow`.

The following sub-arguments are supported:

- a string literal is used to give the shadow struct a user-specified name
- **`off`** disables de/serialization support altogether
- **`attributes(...)`** - custom [attributes](#attrs_family) to be applied to the shadow struct
- **`public(...)`**, **`private`** – specify [visibility](#visibility) of the shadow struct
- **`serialize`** - enable or disable (`serialize(off)`) serialization support for the struct
- **`deserialize`** - enable or disable (`deserialize(off)`) deserialization support for the struct
- **`default`** - wether `serde` must use defaults for missing fields and, perhaps, where to take the defaults from\
- **`forward_attrs`** - a list of field attributes that are to be forwarded to the corresponding field of the shadow
  struct

###### _Notes about `default`_

Valid arguments for the sub-argument are:

* a string literal that has the same meaning as for
  [the container-level `serde` attribute `default`](https://serde.rs/container-attrs.html#default--path)
* a path to a symbol that is bound to an instance of our type: `my_crate::FOO_DEFAULT`
* a call-like path that'd be used literally: `Self::serde_default()`

The last option is preferable because `fieldx` will parse it and replace any found `Self` reference with the
actual structure name making possible future renaming of it much easier.

There is a potentially useful "trick" in how `default` works. Internally, whatever type is returned by the
sub-argument it gets converted into the shadow type with trait [`Into`]. This allows you to use the original struct
as the trait implementation is automatically generated for it. See this example from a test:

```rust
#[cfg(feature = "serde")]
#[fxstruct(sync, get, serde("BazDup", default(Self::serde_default())))]
#[derive(Clone)]
pub(super) struct Baz {
    #[fieldx(reader)]
    f1: String,
    f2: String,
}

impl Baz {
    fn serde_default() -> Fubar {
        Fubar {
            postfix: "from fubar".into()
        }
    }
}

struct Fubar {
    postfix: String,
}

impl From<Fubar> for BazDup {
    fn from(value: Fubar) -> Self {
        Self {
            f1: format!("f1 {}", value.postfix),
            f2: format!("f2 {}", value.postfix),
        }
    }
}

let json_src = r#"{"f1": "f1 json"}"#;
let foo_de = serde_json::from_str::<Baz>(&json_src).expect("Bar deserialization failure");
assert_eq!(*foo_de.f1(), "f1 json".to_string());
assert_eq!(*foo_de.f2(), "f2 from fubar".to_string());
```

### Arguments of `fieldx`

At this point, it's worth refreshing your memory about [sub-arguments of helpers](#sub_args) and how they differ in
semantics between `fxstruct` and `fieldx` attributes.

#### **`skip`**

**Type**: flag

Leave this field alone. The only respected argument of `fieldx` when skipped is the `default`.

#### **`lazy`**

**Type**: helper

Mark field as lazy.

#### **inner_mut***

**Type**: keyword

Enables field interior mutability.

#### **`rename`**

**Type**: function

Specify alternative name for the field. The alternative will be used to form method names and, with `serde` feature
enabled, serialization name[^unless_in_serde].

[^unless_in_serde]: Unless a different alternative name is specified for serialization with `serde` argument.

#### **`get`**, **`get_mut`**, **`set`**, **`reader`**, **`writer`**, **`clearer`**, **`predicate`**, **`optional`**

**Type**: helper

Have similar syntax and semantics to corresponding `fxstruct` arguments:

- [`get`](#get)
- [`get_mut`](#get_mut)
- [`set`](#set)
- [`reader` and `writer`](#reader-writer)
- [`clearer`](#clearer)
- [`predicate`](#predicate)
- [`optional`](#optional)

#### **`optional`**

**Type**: keyword

Explicitly mark field as optional even if neither `predicate` nor `clearer` are requested.

#### **`public(...)`**, **`private`**

Field-default visibility for helper methods. See [the sub-arguments section](#sub_args) above for more details.

#### **`serde`**

**Type**: function

At the field-level this option acts mostly the same way, as [at the struct-level](#serde). With a couple of
differences:

- string literal sub-argument is bypassed into `serde` [field-level `rename`](https://serde.rs/field-attrs.html#rename)
- `default` is responsible for field default value; contrary to the struct-level, it doesn't use [`Into`] trait
- `attributes` will be applied to the field itself
- `serialize`/`deserialize` control field marshalling

#### **`into`**

**Type**: keyword

Sets default for `set` and `builder` arguments.

#### **`builder`**

**Type**: helper

Mostly identical to the [struct-level `builder`](#builder). Field specifics are:

- no `attributes_impl` and `opt_in` (consumed, but ignored)
- string literal specifies setter method name of the builder type for this field
- `attributes` and `attributes_fn` are correspondingly applies to builder field and builder setter method

Field level only argument:

- **`required`** – this field must always get a value from the builder even if otherwise it'd be optional

<a id="about_default"></a>
## Do We Need The `Default` Trait?

Unless explicit `default` argument is used with the `fxstruct` attribute, `fieldx` tries to avoid implementing the
`Default` trait unless really required. Here is the conditions which determine if the implementation is needed:

1. Method `new` is generated by the procedural macro.

   This is, actually, the default behavior which is disabled with [`no_new`](#no_new) argument of the `fxstruct`
   attribute.
1. A field is given a [`default`](#default) value.
1. The struct is `sync` and has a lazy field.

<a id="accessor_vs_reader_writer"></a>
## Why `get`/`get_mut` and `reader`/`writer` For Sync Structs?

It may be confusing at first as to why there are, basically, two different kinds of accessors for sync structs. But
there are reasons for it.

First of all, let's take into account these important factors:

- fields, that are [protected](#protected_unprotected_fields), cannot provide their values directly; lock-guards are
  required for this
- lazy fields are expected to always get some value when read from

Let's focus on a case of lazy fields. They have all properties of lock-protected and optional fields, so we loose
nothing in the context of the `get`/`get_mut` and `reader`/`writer` differences.

### `get` vs `reader`

A bare bones `get` accessor helper is the same thing, as the `reader` helper[^get_reader_guts]. But, as soon as a
user decides that they want `copy` or `clone` accessor behavior, `reader` becomes the only means of reaching out
to field's lock-guard:

[^get_reader_guts]: As a matter of fact, internally they even use the same method-generation code.

```rust
#[fxstruct(sync)]
struct Foo {
    #[fieldx(get(copy), reader, lazy)]
    bar: u32
}
impl Foo {
    fn build_bar(&self) -> u32 { 1234 }
    fn do_something(&self) -> u32 {
        // We need to protect the field value until we're done using it.
        let bar_guard = self.read_bar();
        let outcome = *bar_guard * 2;
        outcome
    }
}
let foo = Foo::new();
assert_eq!(foo.do_something(), 2468);
```

### `get_mut` vs `writer`

This case if significantly different. Despite both helpers are responsible for mutating fields, the `get_mut` helper
remains an accessor in first place, whereas the `writer` is not. In the context of lazy fields it means that
`get_mut` guarantees the field to be initialized first. Then we can mutate its value.

`writer`, instead, provides direct and immediate access to the field's container. It allows to store a value into it
without the builder method to be involved. Since building a lazy field can be expensive, it could be helpful to
avoid it in cases when we don't actually need it[^sync_writer_vs_builder].

[^sync_writer_vs_builder]: Sometimes, if the value is known before a struct instance is created, it might make sense
to use the builder instead of the writer.

Basically, the guard returned by the `writer` helper can only do two things: store an entire value into the field,
and clear the field.

```rust
#[fxstruct(sync)]
struct Foo {
    #[fieldx(get_mut, get(copy), writer, lazy)]
    bar: u32
}
impl Foo {
    fn build_bar(&self) -> u32 {
        eprintln!("Building bar");
        1234
    }
    fn do_something1(&self) {
        eprintln!("Using writer.");
        let mut bar_guard = self.write_bar();
        bar_guard.store(42);
    }
    fn do_something2(&self) {
        eprintln!("Using get_mut.");
        let mut bar_guard = self.bar_mut();
        *bar_guard = 12;
    }
}

let foo = Foo::new();
foo.do_something1();
assert_eq!(foo.bar(), 42);

let foo = Foo::new();
foo.do_something2();
assert_eq!(foo.bar(), 12);
```

This example is expected to output something like this:

```rust
Using writer.
Using get_mut.
Building bar
```

As you can see, use of the `bar_mut` accessor results in the `build_bar` method invoked.

## The Inner Workings

As it was mentioned in the [Basics](#basics) section, `fieldx` rewrites structures with `fxstruct` applied. The
following table reveals the final types of fields. `T` in the table represents the original field type, as specified
by the user; `O` is the original struct type.

| Field Parameters | Plain Type | Sync Type | Async Type |
|------------------|---------------|-----------|-----------|
| `lazy` | `OnceCell<T>` | [`FXProxySync<O, T>`] | [`FXProxyAsync<O,T>`] |
| `optional` (also activated with `clearer` and `proxy`) | `Option<T>` | [`FXRwLockSync<Option<T>>`][`sync::FXRwLockSync`] | [`FXRwLockAsync<Option<T>>`][`async::FXRwLockAsync`] |
| `lock`, `reader` and/or `writer` | N/A | [`FXRwLockSync<T>`][`sync::FXRwLockSync`] | [`FXRwLockAsync<T>`][`async::FXRwLockAsync`] |

Apparently, skipped fields retain their original type. Sure enough, if such a field is of non-`Send` or non-`Sync`
type the entire struct would be missing these traits despite all the efforts from the `fxstruct` macro.

There is also a difference in how the initialization of `lazy` fields is implemented. For plain fields this is done
directly in their accessor methods. Sync structs delegate this functionality to the [`FXProxySync`] type.

### Traits

`fieldx` additionally implement traits `FXStructNonSync` and `FXStructSync` for corresponding kind of structs. Both
traits are empty and only used to distinguish structs from non-`fieldx` ones and from each other. For both of them
`FXStruct` is a super-trait.

### Sync Primitives

The functionality of `sync` structs are backed by primitives provided by the [`parking_lot`] crate.

## Support Of De-/Serialization With `serde`

Transparently de-/serializing container types is a non-trivial task. Luckily, [`serde`] allows us to use special
parameters [`from`](https://serde.rs/container-attrs.html#from) and
[`into`](https://serde.rs/container-attrs.html#into) to perform indirect marshalling via a shadow struct. The way
this functionality implemented by `serde` (and it is for a good reason) requires our original struct to implement
the [`Clone`] trait. `fxstruct` doesn't automatically add a `#[derive(Clone)]` because implementing the trait
might require manual work from the user.

Normally one doesn't need to interfere with the marshalling process. But if such a need emerges then the following
implementation details might be helpful to know about:

- shadow struct mirror-fields of lazy and optional originals are [`Option`]-wrapped
- the struct may be given a custom name using string literal sub-argument of [the `serde` argument](#serde_struct)
- a shadow field may share its attributes with the original if they are listed in `forward_attrs` sub-argument of
  the `serde` argument
- `forward_attrs` is always applied to the fields, no matter if it is used with struct- or field-level `serde`
  argument
- if you need custom attributes applied to the shadow struct, use the `attributes*`-family of `serde` sub-arguments
- same is about non-shared field-level custom attributes: they are to be declared with field-level `attributes*` of
  `serde`

[`parking_lot`]: https://docs.rs/parking_lot
[`serde`]: https://docs.rs/serde

# License

Licensed under [the BSD 3-Clause License](/LICENSE).
