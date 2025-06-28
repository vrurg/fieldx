<!-- markdownlint-disable MD007 MD009 MD012 MD024 MD033 -->
# Changelog

## v0.2.1 - 2025-06-28

### Documentation

- Fix errors in the documentation 
- Add the Further Reading section to the main page/README 
- Give each workspace member its own README 

### Maintenance

- Add homepage to the cargo manifests 
- Add dry run info to the publish question to user 
- Try to only use specific publish feature set for fieldx package 

## v0.2.0 - 2025-06-27

### Features

- Allow optional alternative async-lock instead of tokio

    It’s been long planned to relax the crate’s dependency on
    `tokio`. You can now use the `async-lock` crate to implement `RwLock`
    and `OnceCell`. This change also updates the feature flags: the `async`
    feature must be paired with either `tokio-backend` or
    `async-lock-backend`. Alternatively, enable async support and select an
    implementation in one step with the umbrella flags `async-tokio` or
    `async-lock`.

    `tokio` remains in the requirements for tests.
 
- Add support for `clonable-lock` feature flag

    With this flag, `FXRwLock<T>` types from either `sync` or
    `async` modules will be used instead of `RwLock<T>` from `parking_lot`,
    `tokio`, or `async-lock` crates.
 

### Bug Fixes

- Don't use unsound mapped rwlock guards

    Replace them with FXProxyReadGuard and FXProxyWriteGuard
    wrappers.
 
- Builder's `prefix` must be struct level only sub-argument 
- `prefix` was disrespected for field builder method name 
- Build Docker image for nightly Rust 
- Sync mode is not set for a field when `lock` is used 

### Refactor

- ️‼️ **breaking** Get rid of Sync/Async suffixes in type names where possible

    Use namespaces to address the types. I.e. `sync::FXProxy`,
 
    - ⚠️  The last breaking change before the 0.2.0 release.

### Documentation

- Release the 'FieldX Object Manager' book 

### Testing

- Add some test descriptions 
- Update compilation tests for changes in feature flags and MSRV 
- Implement parallelized testing with Docker

    `cargo make` targets `test-versions` and `update-versions`
    are rather heavy both runtime- and space-wise. Now each version of Rust
    toolchain we test for will be tests in its own Docker container and all
    versions will be run in parallel.
 
- Add examples to the testing 
- Better support for containerization of update-versions target 

### Maintenance

- Bump MSRV to 1.78 
- Another attempt to fix error reporting in the Makefile.toml 
- Skip compilation tests for nightly at the earliest possible stage 

## v0.2.0-beta.1 - 2025-06-16

### Documentation

- Reflect some of the previous versions changes 

### Testing

- Propagate reace condition fix from sync.rs test to sync_generics.rs 

## v0.1.19 - 2025-06-06

### Bug Fixes

- Quick fix for an overlooked case with fallible

    The `fallible` argument makes sense when used with `lazy`,
    which can now be used with both `lock` and `inner_mut`. However, the
    combination of `fallible` with these two was unconditionally prohibited.
 
- Take care of new warnings from the nightly compiler 
- One more location where the new lifetime warning springs up 

## v0.1.18 - 2025-06-06

### Features

- ️‼️ **breaking** Getter is not give by default only with the `lazy` attribute 
    - ⚠️  The reasons why getter was given with `clearer`,
         `predicate`, and `inner_mut` are now less clear than they used to be.
         There is a good reason to get it with `lazy`: unless it combined with
         `lock`, the only way get lazy initialization is to use the getter.
- Implement simplified `lazy` implementation for non-lock fields

    It is not always necessary to lock-protect lazy fields; this
    is especially true for read-only ones. Therefore, unless the `lazy`
    attribute is accompanied by the `lock` or `inner_mut` attributes (which
    are just aliases in the `sync` and `async` modes), lazy evaluation will
    be implemented using `OnceCell` from either `once_cell` or `tokio` for
    the `sync` and `async` contexts, respectively.
 

### Bug Fixes

- `get(as_ref)` wasn't respected for `sync` fields 
- Add now required `get` attribute 

### Refactor

- Move plain-related decls into fieldx::plain module 

### Documentation

- Fix an autocompletion error 
- Add notes on `lazy` enabling the accessor methods 

## v0.1.17 - 2025-06-01

### Features

- Make vast parts of fieldx_derive reusable via fieldx_core crate 
- Add support for associated types to `FXImplConstructor` 
- Make doc_props macro public 
- Implement ToTokens trait for some key types 
- Allow keyword args to also accept a `foo()` syntax

    Previously, this syntax triggered 'expected ...' errors. While
    it worked for manually written code, it failed for token streams
    produced by the same types when wrapped in `FXNestingAttr`, because
    their keyword forms generated the aforementioned function-call-like
    syntax.
 
- Implement FXSetState trait for FXProp

    This allowed cleaning up some trait implementations. So, where
    previously `a_prop.into()` was used, it is now `a_prop.is_set()`.
 
- Implement `to_arg_tokens` method for FXFieldReceiver 

### Bug Fixes

- Clarify the use of reference counted self

    Don't perform extra checks on the validity of the weak
    self-reference unless necessary, and call `post_build` on the object
    itself rather than on its reference-counting container.
 
- Incorrect code generation for async locks 

### Refactor

- Make the codegen context generic over implementation details

    Now what is related to the particular code generation is
    available via the `impl_ctx` field and corresponding method.
 

## v0.1.16 - 2025-04-30

### Features

- Improve functionality of FXSynTuple

    Allow it to take a wider range of Parse-implementing types.
 

### Bug Fixes

- Add missing FXSetState impl for FXSynTuple and FXPunctuated 
- Silence clippy warnings about introduced type complexity 
- Silence some more clippy lints 

### Refactor

- Remove redundant delegations 
- Stop returning a reference by field receiver `span` method 

### Testing

- Complete unfinished tests 

## v0.1.15 - 2025-04-23

### Features

- Add support for explicit builder Default trait implementation

    The builder type implements the Default trait by default.
    If this behavior is undesired, use `builder(default(off))` instead.
 

### Bug Fixes

- Unrequested `Default` impl when `serde` feature is enabled 

### Maintenance

- Make clippy happy with its defaults 

## v0.1.14 - 2025-04-19

### Bug Fixes

- Fix a regression that broke compatibility with fieldx_plus crate 

## v0.1.13 - 2025-04-18

### Features

- Allow accessor mode flags to take the `off` subargument

    This allows to override struct-level defaults for `copy`,
    `clone`, and `as_ref` flags.
 
- Don't require `lock` field types to implement Default 
- Introduce `new` helper and deprecate `no_new`

    This also uncouples `new` from `default`, allowing for use of
    the `Drop` trait with `fieldx` structs.
 

### Bug Fixes

- An old bug in the `sync` test 
- `myself` method in post_build of ref-counted structs

    Previously, `post_build` was invoked on reference-counted
    objects before they were wrapped in `Rc` or `Arc`. This led to the
    failure of the `myself` method because the weak reference to self
    remained uninitialized.
 
- Superfluous warning on unused builder methods

    Mark field builder methods as `#[allow(unused)]` if
    corresponding fields are optional or have alternative sources of values
    (such as lazy initialization or default values).
 
- Regression that broke non-debug build profiles 
- Compiler error message location for post_build methods 

### Documentation

- Fix CHANGELOG template 
- Document the `new` helper argument 

### Testing

- Remove laziness from attribute to make test working again

    Builder methods of lazy fields are now marked with
    `#[allow(unused)]` to avoid unused warnings.
 

### ️ Miscellaneous Tasks

- Don't include release commit message in changelog 

## v0.1.12 - 2025-03-21

### Bug Fixes

- Missing `documentation` key in crates metadata 

### ️ Miscellaneous Tasks

- Release fieldx version 0.1.12 

## v0.1.11 - 2025-03-21

### Features

- Generate errors for useless sub-arguments 
- Add `doc` argument to document generated code

    And preserve field doc comments for accessors and builder
    setters.
 
- Restrict use of more subargs at field level

    This feature came along with refactoring aiming at unification
    of internal interfaces.
 

### Bug Fixes

- Clearer treating a field as a lock on `sync` structure

    This shouldn't happen without explicit `lock` being set.
    This is a bug inherited from the times when `optional` on a `sync`
    struct was implying the `lock` argument.
 
- Incorrect hanlding of sync+optional by serialization 
- Option<FXProp> span not updated from FXNestingAttr wrapper 
- Recover lost functionality of prefixing builder setter methods

    While it was the right move to change the functionality of the
    `builder` argument's literal sub-argument to define the builder's struct
    name, the ability to bulk-assign prefixes to builder setter names was
    lost. It is now recovered by introducing the `prefix` sub-argument to
    the `builder` argument.
 

### Refactor

- ️‼️ **breaking** Improve how generated code is bound to the source

    With this commit the following breaking changes are introduced:
 
    - ⚠️  `public` argument is gone, use `vis(...)` instead
    - ⚠️  struct-level `serde` literal string argument is now
         used as `rename` argument of `#[serde()]` attribute.
    - ⚠️  `optional` is not implying `lock` for `sync` structs
         anymore. Explitcit `lock` argument may be required.
- Major step towards constructor-based architecture

    No extra features added, just refactoring the codebase to
    make it more maintainable and extensible. If no big bugs are found, then
    this release could serve as a stabilization prior to the v0.2.0 release.
 

### Documentation

- Document the latest changes and fix some errors 

### ️ Miscellaneous Tasks

- Release fieldx version 0.1.11 

## fieldx-v0.1.10 - 2025-02-22

### Features

- Allow `default` to take expressions as arguments 

### Bug Fixes

- Insufficiently strict handling of 'skip' 
- Lazy fields picking up `optional` from struct level 

## fieldx-v0.1.9 - 2025-01-16

### Features

- Allow inner_mut to be used in sync mode 
- Allow wider use of inner_mut 

### Bug Fixes

- Remove erroneous `documentation` field from `Cargo.toml` 
- Builder methods visibility must not always be public 
- Regression, parking_lot types must be re-exported 
- Incorrect codegen for const generic params with default 
- Generics for serde shadow struct 
- Incorrect generation of serde shadow Default implementation 
- Incorrect generic handling in Self fixup 
- Sanitize the logic for choosing field concurrency mode 

## fieldx-v0.1.8 - 2024-12-05

### Features

- Implement `async` support 
- Allow use of `async` keyword 
- Implement support for fallible lazy builders 
- Support for builder's custom error type 
- Introduce 'sync' and 'async' features 

### Bug Fixes

- Implement `Clone` for `FXProxyAsync` 
- Error diagnostics for serde-related code 
- Fieldx_derive must depend on fieldx by path in dev-deps 

### Refactor

- Rename internal structs for the sake of naming consistency 

### Documentation

- Update docs with `async` addition 
- Completed documenting `fieldx_aux` crate 

### Testing

- Only test failing compiles under the Makefile.toml environment 
- Fix testing documentation examples of `fieldx_derive` crate 

### Fix

- Don't implement Default for shadow if it's not needed 

### Styling

- Format all sources 

## fieldx-v0.1.7 - 2024-11-22

### Features

- Implement builder `init` argument 

### Documentation

- Document builder `init` argument 

## fieldx-v0.1.6 - 2024-10-19

### Features

- Allow field's `default` to be just a keyword so it would fallback to `Default::default()` 
- Make builder setter methods to use more common 'self ownership' scheme instead of using `&mut` 

### Bug Fixes

- Avoid function name case warning 
- Reduce builder dependency on Default 
- Allow non-snake-case names for generated serde methods 

### ️ Miscellaneous Tasks

- Release fieldx version 0.1.5 

## fieldx-v0.1.5 - 2024-10-03

### Features

- Complete implementation of reference counted objects 
- Make builder's `into` argument accept `into(off)` form 
- Add support for `builder(required)` 
- Implement inner mutability pattern 
- Implement struct-level `builder(opt_in)` 
- Allow better granularity over fields concurrency mode 
- Implement PartialEq and Eq 
- Added two convenice types: FXSynValueArg and FXSynTupleArg 
- Added implementation of FXPunctuated 

### Bug Fixes

- Marshalling of optional fields 
- Fix a thinko in serde deserialization of optionals 
- Suppress a harmless warning 
- Remove unused import 
- Improve some error location reporting 
- Propagate "diagnostics" feature to the darling crate 

### Refactor

- Make more types available via fieldx_aux 
- Split fxproxy proxy module into submodules 
- Get rid of FXStructSync and FXStructNonSync 
- Removed unused struct 

### Documentation

- Describe interior mutability pattern 

### ️ Miscellaneous Tasks

- Release fieldx_derive_support version 0.1.4 
- Release fieldx_aux version 0.1.4 
- Release fieldx_derive version 0.1.4 
- Release fieldx version 0.1.4 
- Release fieldx_derive_support version 0.1.5 
- Release fieldx_aux version 0.1.5 
- Release fieldx_derive version 0.1.5 
- Release fieldx version 0.1.5 

## fieldx-v0.1.3 - 2024-08-02

### Features

- Add feature `send_guard` 
- Support reference counted objects 

### Documentation

- Document the new `rc` argument and crate features 

### ️ Miscellaneous Tasks

- Release fieldx version 0.1.3 

## fieldx-v0.1.2 - 2024-06-19

### Features

- ️‼️ **breaking** Allow optional unlocked fields on sync structs 
- Add support for `attributes` and `attributes_impl` for `fxstruct` 

### Bug Fixes

- Make sure that Copy trait bound check doesn't fail for generics 

### Documentation

- Document new argument `lock` 
- Document `attributes` and `attributes_impl` of `fxstruct` 

### Testing

- Streamline toolchain(version)-dependent testing 
- Use stricter/safer atomic ordering 
- Refactor tests to get rid of warnings 

### Maintenance

- Set some environment variables conditionally 
- *(CI)* Exclude `nightly` toolchain from testing under `windows` 
- *(cliff)* Allow scoping for `feat`, `fix`, and `maint` groups 

### ️ Miscellaneous Tasks

- Release fieldx version 0.1.2 

### Main

- Should've not use `publish` with `cargo release` 

## fieldx-v0.1.1 - 2024-06-02

### Features

- ️‼️ **breaking** Full support for accessors for sync structs and `lock` argument 
    - ⚠️  new accessors are incompatible with the previous approach

### Documentation

- Document the changes, related to the accessors of sync structs 

### Testing

- Adjusted tests for the new accessors concept and `lock` 

### Maintenance

- Fix incorrect handling of environment variables in Makefile.toml 
- Fix generation of CHANGELOG by `cargo release` 
- Use `cargo release` for the `publish` target 

## v0.1.0 - 2024-05-31

<!-- generated by git-cliff -->
