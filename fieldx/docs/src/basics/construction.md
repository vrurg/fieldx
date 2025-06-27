# Construction

There are two and a half ways to instantiate a struct in FieldX:

1. By using its [default](./default.md)
    - by using its {{i:new}} method, which is just syntax sugar around the `Default` trait under the hood. If you don't want the implicit `new` method then just do:

        ```rust,ignore
        #[fxstruct(new(off))]
        ```

1. By using [the {{i:builder pattern}}](./terminology.md#builder).

Since there is nothing more to be added about the `new()/default()`, let's focus on the builder pattern.
