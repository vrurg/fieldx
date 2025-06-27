# Feature Flags

The following {{i:feature flag}}s are supported by this crate:

| *Feature* | *Description* |
|-|-|
| `sync` | Support for sync-safe mode of operation |
| `async` | Support for async mode of operation |
| `tokio-backend` | Selects the Tokio backend for async mode. A no-op without the `async` feature. |
| `async-lock-backend` | Selects the `async-lock` backend for async mode. A no-op without the `async` feature. |
| {{i:`async-tokio`}} | Combines `async` and `tokio-backend` features. |
| {{i:`async-lock`}} | Combines `async` and `async-lock-backend` features. |
| {{i:`clonable-lock`}} | Enables the [clonable lock wrapper type](more_on_locks.md). |
| `send_guard` | See corresponding feature of the [`parking_lot` crate](https://crates.io/crates/parking_lot) |
| `serde` | Enable support for `serde` marshalling. |
| `diagnostics` | Enable additional diagnostics for compile time errors. Experimental, requires Rust nightly toolset. |

```admonish warning
The `tokio-backend` and `async-lock-backend` features are mutually exclusive. You can only use one of them at a time or FieldX will produce a compile-time error.
```
