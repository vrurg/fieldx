# Optional Values

There is no need to explain what [`Option`](https://doc.rust-lang.org/std/option/enum.Option.html) is in Rust and what role it plays. FieldX pays a bit of extra attention to it, though, because the concepts of {{i:optional values}} partially propagate into other patterns implemented by the crate. So, let's take this snippet as an example:

```rust ignore
#[fxstruct]
struct Foo {
    #[fieldx(optional, get)]
    value: String,
}
```

It is quite apparent that the `value` field is optional and its final type is `Option<String>`. This is what a user gets. But along the lines, FieldX will use this information to generate proper code where its semantics depend on the optionality of the field. One case would be the builder implementation not failing with an error if an optional field is not set.

## Predicate{{hi:predicate}}

In this chapter, we will discuss functionality that is more closely tied to optional values. Let's get back to the fact that there may be no value in the field. Normally, we'd check this with the `is_none()` method, but FieldX allows us to give the public API that our implementation provides a little touch of beauty:

```rust ignore
#[fxstruct]
struct Foo {
    #[fieldx(optional, predicate, get)]
    value: String,
}
```

This gives us a `has_value()` method that returns `true` if the field is set:

```rust ignore
let foo = Foo::new();
assert!(!foo.has_value());
```

Not satisfied with the name or it doesn't fit your API standards? `predicate` is a helper argument; so, no problem — give it another name![^no_optional_keyword]

```rust ignore
#[fxstruct]
struct Foo {
    #[fieldx(predicate("we_are_good"), get, set)]
    value: String,
}
```

And, of course:

```rust ignore
let mut foo = Foo::new();
foo.set_value("Hello, world!".to_string());
assert!(foo.we_are_good());
```

This sample demonstrates another little perk of declaring optional fields with FieldX: there is no need to wrap the argument of the setter method into `Some()`, as it would be necessary with the explicit `Option<String>` approach.

## Clearer

Where a value can be given, it can also be taken away. This is what the `{{i:clearer}}` argument is for[^no_optional_keyword]:

```rust ignore
#[fxstruct]
struct Foo {
    #[fieldx(clearer, get)]
    value: String,
}
```

Let's combine this with the `predicate` argument and see how it works:

```rust ignore
{{#include ../../../examples/book_optional.rs:decl}}

{{#include ../../../examples/book_optional.rs:usage}}
```

Since `clearer` is a helper too, it can be renamed as well:

```rust ignore
#[fxstruct]
struct Foo {
    #[fieldx(clearer("reset_value"), predicate, get)]
    value: String,
}
```

[^no_optional_keyword]: Note that the `optional` keyword is omitted in this case because `predicate` and `clear` arguments imply that the field is optional.

## AsRef

Since by default  the accessor methods return a reference to the field value, it is sometimes (often) gives us a situation where in order to do something with the `Option` it returns we need to convert it from `&Option<T>` to `Option<&T>`. Calling the `as_ref()` method every time is a bit tedious, so FieldX provides sub-argument {{i:`as_ref`}} for the `get` argument that does this for us automatically:

```rust ignore
{{#include ../../../examples/book_optional.rs:as_ref_decl}}

{{#include ../../../examples/book_optional.rs:as_ref_usage}}
```

## Builder Pattern {{hi:builder pattern}}

Here is another reason why the `optional` keyword makes sense on its own. When generating the {{i:builder type}}, FieldX pays the same attention to the optionality of a field as it does to its [default value](./construction.md#default-value):

```rust ignore
{{#include ../../../examples/book_builder_pattern.rs:opt_decl}}

{{#include ../../../examples/book_builder_pattern.rs:opt_usage}}
```

About the same result could be achieved with an explicit `Option`-typed field by giving it the explicit `default(None)` argument, but doesn't the above sample look way better in its conciseness and readability?
