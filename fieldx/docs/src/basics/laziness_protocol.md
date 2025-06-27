# Laziness Protocol

The implementation of lazy field initialization in FieldX is based on a few conventions between the crate and the user. These conventions are referred to as the "{{i:laziness protocol}}". The conventions are as follows:

1. For any uninitialized field, the initialization takes place exactly once, when the field is first read from, regardless of the field's [mode of operation](./modes_of_operation.md).
1. The value for the initialization is provided by a lazy builder method, which is supplied by the user.
1. FieldX guarantees that in sync or async modes of operation, rule #1 also implies that the builder method is called exactly once.
1. The builder method is expected not to mutate its object unless it is unavoidable and done with the utmost care.
1. The builder method is expected to be infallible unless otherwise specified by the user.

There is no need to duplicate the example from the [Introduction](../intro/example.md) here since it already demonstrates the laziness protocol in action. Let's discuss a few other aspects here.

## Fallible Builders{{hi:fallible builder}}

The ideal world has no errors that we must deal with. However, our world is far from ideal, hence sometimes a builder method may fail, and there must be a way to propagate this failure. For example, in the [Lazy Field Initialization](./lazy_field_initialization.md) section, we use a hypothetical case where data is pulled from a network resource. The pseudo-code we used doesn't account for the possibility of a network failure or any other error that might occur during the data retrieval process. Let's address this problem now.

```rust ignore
#[fxstruct(lazy, fallible(off, error(AppError)))]
struct NetworkResource {
    #[fieldx(fallible)]
    data: DataType,
    #[fieldx(fallible)]
    location_directory: LocationDirectory,
}

impl NetworkResource {
    fn build_location_directory(&self) -> Result<LocationDirectory, AppError> {
        Ok(self.network_request()?.get_location_directory()?)
    }

    fn build_data(&self) -> Result<DataType, AppError> {
        let location = self.location_directory()?.get_data_location();
        Ok(self.network_request()?.get_data(location)?)
    }
}
```

```admonish info
In this code, we assume that the `network_request` method, as well as the `get_*` methods we call on its results, return an error type for which `AppError` at least implements the `From` trait.
```

Here is what's happening in this code:

1. First of all, all fields of the struct are marked as lazily initialized.
1. Then we specify that the default error type for all fallible fields is `AppError`. This is a little trick where we use the fact that field-level properties get their defaults from the struct level. The declaration `fallible(off, error(AppError))` means that we turn off the fallible mode for all fields by default but set the default error type to `AppError`.
1. Then we mark fields that we want to be fallible. Without the struct-level default, we'd have to write `fallible(error(AppError))` for each field.
1. Finally, we implement the builder methods for the fields. The methods return a `Result` type with `AppError` as the error type.

It really takes more time and words to explain all this than to actually write the code...

OK, down to the usage. In our application code, we can now do the following:

```rust ignore
let resource = NetworkResource::new();
match resource.data() {
    Ok(data) => {
        // Use the data
    }
    Err(err) => {
        // Handle the error
    }
}
```

Considering the network resource is likely to be stored in a field of some other struct, which would need the `data` in one of its methods, it may take the following form:

```rust ignore
fn do_something(&self) -> Result<(), AppError> {
    let resource = self.network_resource()?;
    let data = resource.data()?;
    // Do something with the data
    Ok(())
}
```

The pivotal change in the API of the `NetworkResource` implementation is that the `data()` accessor now returns a `Result<DataType, AppError>`. Otherwise, the usage remains the same as before; i.e., we still can use `copy` or `clone` sub-arguments, and so on.
