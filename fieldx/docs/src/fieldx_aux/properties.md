# Properties

We start with one of the simplest types that, although basic, is key for proper error reporting.

A {{i:property}} in FieldX is a pair consisting of a value of an arbitrary type and an optional span attached to it. On most occasions, when code is generated using a user-provided value or when the kind of code produced depends on a flag being set or unset, FieldX uses the span associated with the value or flag. For example, in pseudo-code, it may look like this:

```rust ignore
let default_span = props.accessor().final_span();
let is_lazy = props.lazy();
let token_stream = if *lazy {
    quote_spanned! {lazy.final_span()=>
        // Code for lazy initialization
    }
}
else {
    quote_spanned! {default_span=>
        // Code for non-lazy initialization
    }
}
```

Let's examine what's happening here.

1. Both methods `accessor()` and `lazy()` return a [`fieldx_aux::property::FXProp`] value. In this example, both are `FXProp<bool>`, but for the accessor, the type is irrelevant in the context of this chapter.
2. `FXProp` dereferences into the value it holds.
3. `final_span()` returns a [`proc_macro2::Span`] value. While the span on a property is optional, the `final_span()` method always returns a valid span by using `Span::call_site()` as a fallback.

If there is an error in the generated code, the compiler will correctly point to the code location that caused the error. For instance, if the source of the problem is the `lazy` argument, the reported error might look like this:

```stdout
  |
  | #[fxstruct(lazy, optional)]
  |            ^^^^
```

What is more interesting than how the properties are used is how they are created. A property allows us to track the location in the user source code where a value or flag originated, regardless of how many steps or transformations it has undergone. Consider this list from the type documentation, which describes how the decision to implement the `Default` trait for a struct is made:

> 1. The explicit `default` argument at the struct level.
> 2. An explicit `default` argument of a field.
> 3. An explicit `default` sub-argument of a field `serde` argument.
> 4. A lazy attribute of a sync-mode field.
> 5. Disabled deserialization of a field while struct deserialization is enabled.

As you can see, a simple boolean that provides a "yes" or "no" answer can originate from the struct-level `fxstruct` attribute or the field-level `fieldx` attribute. It can be explicitly declared or derived from a sub-argument. In terms of the FieldX core implementation, this involves a complex process of merging struct- and field-level arguments into a field-final set of properties (documented in the [FieldX Core](../fieldx_core.md) chapter). These final properties are then iterated over by the implementation of struct-level properties to determine whether the `Default` trait is needed.

If this sounds overwhelming, don't worry. The last paragraph is here only to illustrate the complexity of preserving origins when all we want is to simplify the user's experience by inferring their intent correctly.

This is where `FXProp` comes into play. For example, here is what FieldX core does when it detects a field with the `default` argument:

```rust,ignore
// has_default: FXProp<bool>
let has_default = fctx.props().field_props().has_default();
if *has_default {
    return Some(has_default);
}
```

```admonish
`FXProp<T>` blanket-implements the `Copy` trait for all `T: Copy`. This is why the code uses it.
```

Or, when determining if deserialization is disabled for a field:

```rust,ignore
// no_deserialize: FXProp<bool>
let no_deserialize = fctx.serde().not().or(fctx.deserialize().not());
if *no_deserialize {
    return Some(no_deserialize);
}
```

In this case, the implementation of the [`fieldx_aux::property::FXPropBool`] trait is used, which provides the `not()` and `or()` methods. The implementation carries span information from either the `off` flag of the `serde` argument or the `off` flag of its `deserialize` sub-argument into `no_deserialize`. If true, this boolean property influences the decision about implementing the `Default` trait.

While boolean properties are often straightforward to use, there are cases where more complexity is required. Even then, the worst-case scenario might look like this:

```rust,ignore
if self.serde_default_value().is_some() {
    return FXProp::new(true, self.serde_default_value().orig_span());
}
```

```admonish
This example is outdated and incorrect! The [`fieldx_aux::traits::FXSetState`] trait is implemented for the [`fieldx_aux::default_arg::FXDefault`] type and provides the `is_set()` method, which returns a true property unless the argument has its `off` flag set. The trait is also implemented for `Option<T: FXSetState>` and returns a false property for `None`.

By now the error is fixed but the example remains here for illustration purposes.
```
