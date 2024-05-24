#![doc(html_root_url = "https://docs.rs/")]
//! Procedural macro for constructing structs with lazily initialized fields, builder pattern, and [`serde`] support
//! with focus on declarative syntax.
//!
//! Let's start with a simple example:
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
//! - a struct with lazy by default fields is declared
//! - laziness is explicitly disabled for field `order`
//! - methods `build_count` and `build_foo` return initial values for corresponding fields
//!
//! At run-time we first ensure that the `order` vector is empty meaning none of the `build_` methods was called. Then
//! we read from `foo` using its accessor method. Then we make sure that each `build_` method was invoked only once.
//!
//! As one can notice, minimal amount of handwork is needed here as most of boilerplate is handled by the macro, which
//! provides even basic `new` associated function.
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
//! - field types could be wrapped into container types
//!
//!   In the above example `foo` and `count` become [`OnceCell<String>`][OnceCell] and `OnceCell<usize>`, whereas
//!   `order` remains unchanged.
//!
//! - partial implementation of `Foo` is added with support methods and associated functions
//!
//!   I.e. this is where accessor methods and `new` live.
//!
//! - depending on parameters, implicit implementation of the [`Default`] trait could be added
//! - if requested, builder struct and `builder` associated function will be implemented
//! - also, if requested, a shadow struct for correct `serde` support will be there too
//!
//! **Note** that user is highly discouraged from directly accessing modified fields. The modules does its best to
//! provided all necessary API via corresponding methods.
//!
//! # Sync And Non-Sync Structs
//!
//! If a thread-safe struct is needed then `fxstruct` must take the `sync` argument: `#[fxstruct(sync, ...)]`. When told
//! so, the macro will do its best to provide concurrency safety at the field level. It means that:
//!
//! - builder methods are guaranteed to be invoked once and exactly once per each lazy initialization, be it single- or
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
//! # Optional Fields <a id="optional_fields"></a>
//!
//! _Optional_ in this context has the same meaning, as in [`Option`] type. Sure thing, one can simply declare a field
//! using the core type (and, as a matter of fact, this is what `fieldx` is using internally anyway). What's the
//! advantages of using `fieldx` then?
//!
//! First of all, manual declaration may mean additional boilerplate code to implement accessor and not only. With
//! `fieldx` most of it can be hidden under a single declaration:
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
//! Though being very simple concept, laziness has its own peculiarities. The basics, as it's been shown above, are such
//! that when we declare a field as `lazy` the macro wraps it into some kind of proxy container type ([`OnceCell`] for
//! non-sync structs). The first read[^only_via_method] from an uninialized field will result in the builder method to
//! be invoked and the value it returns to be stored in the field.
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
//! 1. Builders cannot mutate their objects. This limitation also comes from the fact that a typical accessor method
//!    doesn't need and must not use mutable `&self`. Of course, it is always possible to use internal mutability, as
//!    in the first example here.
//!
//! [^only_via_method]: Apparently, the access has to be made by calling a corresponding method. Mostly it'd be field's
//! accessor, but for `sync` structs it's more likely to be a reader.
//!
//! # Usage
//!
//! Most arguments of both `fxstruct` and `fieldx` can take either of the two forms: a keyword (`arg`), or a
//! *"function"* (`arg(subarg)`).
//!
//! Also, most of the arguments are shared by both `fxstruct` and `fieldx`. But their meaning and the way their
//! arguments are interpreted could be slightly different for each attribute. For example, if an argument takes a
//! literal string subargument it is likely to be a method name when associated with `fieldx`; but for `fxstruct` it
//! would define common prefix for method names.
//!
//! There is also a commonality between most of the arguments: they can be temporarily (say, for testing purposes) or
//! permanently turned off by using `off` sub-argument with them. See `lazy(off)` in the
//! above example.
//!
//! # Attribute Arguments
//!
//! A few words about terminology here: <a id="attr_terminology"></a>
//!
//! - argument **Type** determines what subarguments can be received:
//!   * _keyword_ – boolean-like, only accepts `off`: `keyword(off)`
//!   * _helper_ - introduce functionality that is bound to a helper method (see below)
//!   * _list_ or _function_ – can take multiple subarguments
//!   * _meta_ - can take some syntax constructs
//! - helper method – implements certain functionality
//!
//!   Almost all helpers are generated by the macro. The only exception are lazy builders which must be provided by the
//!   user.
//! - **For** specifies if argument is specific to an attribute
//!
//! ## Sub-arguments of Helper Arguments <a id="sub_args"></a>
//!
//! Helper arguments share a bunch of common sub-arguments. We will describe them here, but if their meaning is unclear
//! it'd be better to skip this section and get back to it later.
//!
//! | Sub-argument | In fxstruct | In fxfield |
//! |-|-|-|
//! | **`off`** | disable helper | disable helper |
//! | a non-empty string literal (**"foo"**) | method name prefix | explicit method name (prefix not used) |
//! | **`attributes_fn`** | default attributes for corresponding kind of helper methods | attributes for field's helper method |
//! | **`public`, `public(crate)`, `public(super)`, `public(some::module)`, `private`** | default visibility | visibility for field helper |
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
//! ## `attributes*` Family of Sub-Arguments <a id="attrs_family"></a>
//!
//! Sometimes it might be necessary to specify attributes for various generated syntax elements like methods, or
//! auxiliary structs. Where applicable, this functionality is supported by `attributes*` (sub)arguments. Their syntax
//! is `attributes(<attr1>, <attr2>, ...)` where an `<attr>` is specified exactly, as it would be specified in the code,
//! but with starting `#[` and finishing `]` being omitted. For example, `attributes_fn(allow(dead_code), cfg(feature =
//! "myfeature"))` will expand into something like:
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
//! ### **`default`***
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
//! only basic helper sub-arguments are accepted.
//!
//! Normally mutable accessors have the same name, as immutable ones, but with `_mut` suffix:
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
//! Clearers return the current field value. If field is already uninialized (or never has been yet) `None` will be
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
//! Explicitly declares a field as optional. Useful when neither predicate nor clearer helpers are needed and yet we'd
//! like to make the field optional.
//!
//! ### **`public(...)`**, **`private`**
//!
//! Specify defaults for helpers. See [the sub-arguments section](#sub_args) above for more details.
//!
//! ### **`clone`**, **`copy`**
//!
//! Specify defaults for accessor helpers.
//!
//! ### **`serde`**
//!
//! **Type**: [function](#attr_terminology)
//!
//! Enabled with `serde` feature, which is off by default.
//!
//! Support for de/serialization will be discussed in more details in a section below. What is important to know at this
//! point is that due to use of container types direct serialization of a struct is hardly possible. Therefore `fieldx`
//! utilizes serde's `into` and `from` by creating a special shadow struct. The shadow is named after the original by
//! prepending the name with double underscore and appending *Shadow* suffix: `__FooShadow`.
//!
//! The following sub-arguments are supported:
//!
//! - a string literal is used to give shadow struct a non-default name
//! - **`off`** disables de/serialization support altogether
//! - **`serialize`** - enable or disable (`serialize(off)`) serialization support for the struct
//! - **`deserialize`** - enable or disable (`deserialize(off)`) deserialization support for the struct
//! - **`forward_attrs`** - a list of field attributes that are to be forwarded to the corresponding field of the shadow struct
//! - **`default`** - wether `serde` must use defaults for missing fields and, perhaps, where to take the defaults from
//!
//! ## Arguments of `fieldx`
//!
//! ... TODO ...
//!
//! # Do We Need The `Default` Trait? <a id="about_default"></a>
//!
// TODO Describe how `no_new`, `default`, and `sync` interact and determine wether `Default` implementation is produced and what happens to the `new` method.
//! ...
//!
//! [`serde`]: https://docs.rs/serde

pub mod errors;
pub mod traits;

pub use fieldx_derive::fxstruct;
#[doc(hidden)]
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::{any, borrow::Borrow, cell::RefCell, fmt::Debug, marker::PhantomData, ops::Deref, sync::atomic::AtomicBool};
#[doc(hidden)]
pub use std::{cell::OnceCell, fmt, sync::atomic::Ordering};
use traits::FXStructSync;

pub struct FXProxy<O, T>
where
    O: FXStructSync,
{
    value:   RwLock<Option<T>>,
    is_set:  AtomicBool,
    builder: RwLock<Option<fn(&O) -> T>>,
}

// We need FXRwLock because RwLock doesn't implement Clone
#[derive(Default)]
pub struct FXRwLock<T>(RwLock<T>);

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
    pub fn new_default(builder_method: fn(&O) -> T, value: Option<T>) -> Self {
        Self {
            is_set:  AtomicBool::new(value.is_some()),
            value:   RwLock::new(value),
            builder: RwLock::new(Some(builder_method)),
        }
    }

    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }

    #[inline]
    fn is_set_raw(&self) -> &AtomicBool {
        &self.is_set
    }

    pub fn is_set(&self) -> bool {
        self.is_set_raw().load(Ordering::SeqCst)
    }

    pub fn read_or_init<'a>(&'a self, owner: &O) -> RwLockReadGuard<'a, Option<T>> {
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
            RwLockWriteGuard::downgrade(wguard)
        }
        else {
            RwLockUpgradableReadGuard::downgrade(guard)
        }
    }

    pub fn read<'a>(&'a self, owner: &O) -> MappedRwLockReadGuard<'a, T> {
        RwLockReadGuard::map(self.read_or_init(owner), |data: &Option<T>| data.as_ref().unwrap())
    }

    pub fn write<'a>(&'a self) -> FXWrLock<'a, O, T> {
        FXWrLock::<'a, O, T>::new(self.value.write(), self)
    }

    fn clear_with_lock(&self, wguard: &mut RwLockWriteGuard<Option<T>>) -> Option<T> {
        self.is_set_raw().store(false, Ordering::SeqCst);
        wguard.take()
    }

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
    pub fn new(lock: RwLockWriteGuard<'a, Option<T>>, fxproxy: &'a FXProxy<O, T>) -> Self {
        let lock = RefCell::new(lock);
        Self {
            lock,
            fxproxy,
            _phantom: PhantomData,
        }
    }

    pub fn store(&mut self, value: T) -> Option<T> {
        self.fxproxy.is_set_raw().store(true, Ordering::Release);
        self.lock.borrow_mut().replace(value)
    }

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
    pub fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T> From<T> for FXRwLock<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(value.into()))
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
