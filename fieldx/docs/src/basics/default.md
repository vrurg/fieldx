# Default

FieldX can provide implicit implementation of the `Default` trait for a struct in one of the following cases:

- When the argument `{{i:default}}` is provided and active to the `fxstruct` macro.

    ```rust,ignore
    #[fxstruct(default)]
    struct Foo {
        is_set: bool,
    }
    ```

- When the argument `default` is provided and active for any field's `fieldx` attribute.

    ```rust,ignore
    #[fxstruct]
    struct Foo {
        #[fieldx(get(copy), default(3.1415926535))]
        pi: f32,
    }

    let foo = Foo::default();
    assert_eq!(foo.pi(), 3.1415926535);
    ```

- When another argument, like the struct level `new` argument, needs it.

    ```rust,ignore
    #[fxstruct(new)]
    struct Bar {
        #[fieldx(get(copy), default(2.7182818284))]
        e: f32,
    }

    let bar = Bar::new();
    assert_eq!(bar.e(), 2.7182818284);
    ```
