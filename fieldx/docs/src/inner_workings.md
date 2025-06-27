# Inner Workings { #inner-workings }

As it was mentioned in the [Basics](basics.md), `fieldx` rewrites structures with `fxstruct` applied. The
following table reveals the final types of fields. `T` in the table represents the original field type, as specified
by the user; `O` is the original struct type.

| Field Parameters | Plain Type | Sync Type | Async Type |
|------------------|---------------|-----------|-----------|
| {{i:`inner_mut`}} | [`RefCell<T>`](std::cell::RefCell) | [`sync::FXRwLock<T>`](`sync::FXRwLock`) | [`async::FXRwLock<T>`](`async::FXRwLock`) |
| {{i:`lazy`}} | [`once_cell::unsync::OnceCell<T>`][`once_cell::unsync::OnceCell`] | [`once_cell::sync::OnceCell<T>`](crate::sync::OnceCell) | [`tokio::sync::OnceCell<T>`](crate::async::OnceCell) |
| `lazy` + {{i:lock}} | _N/A_ | [`sync::FXProxy<O, T>`] | [`async::FXProxy<O,T>`](fieldx::async::FXProxy) |
| {{i:`optional`}} (also activated with `clearer` and `predicate`) | `Option<T>` | [`sync::FXRwLock<Option<T>>`] | [`async::FXRwLock<Option<T>>`](fieldx::async::FXRwLock) |
| `lock`, {{i:`reader`}} and/or {{i:`writer`}} | _N/A_ | [`sync::FXRwLock<T>`](`sync::FXRwLock`) | [`async::FXRwLock<T>`](`async::FXRwLock`) |

Apparently, skipped fields retain their original type. Sure enough, if such a field is of non-`Send` or non-`Sync`
type the entire struct would be missing these traits despite all the efforts from the `fxstruct` macro.

There is also a difference in how the initialization of `lazy` fields is implemented. For non-locked (simple) fields
the lazy builder method is called directly from the accessor method. For locked fields, however, the lazy
builder is invoked by the implementation of the proxy type.
