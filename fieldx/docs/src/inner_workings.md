# Inner Workings { #inner-workings }

As it was mentioned in the [Basics](basics.md), `fieldx` rewrites structures with `fxstruct` applied. The
following table reveals the final types of fields. `T` in the table represents the original field type, as specified
by the user; `O` is the original struct type.

| Field Parameters | Plain Type | Sync Type | Async Type |
|------------------|---------------|-----------|-----------|
| {{i:`inner_mut`}} | [`RefCell<T>`](std::cell::RefCell) | [`sync::FXRwLock<T>`][sync::FXRwLock] or [`parking_lot::RwLock`](https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html) | [`async::FXRwLock<T>`][doc_async::FXRwLock], or [`tokio::sync::RwLock`](https://docs.rs/tokio/latest/tokio/sync/type.RwLock.html) or [`async_lock::RwLock`](https://docs.rs/async_lock/latest/async_lock/struct.RwLock.html) |
| {{i:`lazy`}} | [`once_cell::unsync::OnceCell<T>`][once_cell::unsync::OnceCell] | [`once_cell::sync::OnceCell<T>`][crate::sync::OnceCell] | [`tokio::sync::OnceCell<T>`][tokio::sync::OnceCell] or [`async_lock::OnceCell<T>`](https://docs.rs/async_lock/latest/async_lock/struct.OnceCell.html) |
| `lazy` + {{i:`lock`}} | _N/A_ | [`sync::FXProxy<O, T>`] | [`async::FXProxy<O,T>`][doc_async::FXProxy] |
| {{i:`optional`}} (also activated with `clearer` and `predicate`) | `Option<T>` | [`sync::FXRwLock<Option<T>>`] | [`async::FXRwLock<Option<T>>`][doc_async::FXRwLock] |
| `lock`, {{i:`reader`}} and/or {{i:`writer`}} | _N/A_ | [`sync::FXRwLock<T>`](`sync::FXRwLock`) | [`async::FXRwLock<T>`][doc_async::FXRwLock] |

```admonish info
The way a particular container type is chosen depends on the combination of the enabled feature flags. With regard to the async mode operation refer to the Async Mode of Operation chapter, [Backend Choice](basics/mode_async.md#backend-choice) section for more details. With regard to the choice between `FXRwLock` and `RwLock` see the [More on Locks](more_on_locks.md) chapter.
```

Apparently, skipped fields retain their original type. Sure enough, if such a field is of non-`Send` or non-`Sync`
type the entire struct would be missing these traits despite all the efforts from the `fxstruct` macro.

There is also a difference in how the initialization of `lazy` fields is implemented. For non-locked (simple) fields
the lazy builder method is called directly from the accessor method. For locked fields, however, the lazy
builder is invoked by the implementation of the proxy type.
