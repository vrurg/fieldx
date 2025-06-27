# More On Locks

```admonish warning
Potential deadlock information is provided in this chapter.
```

The Basics chapter on [locks](basics/locks.md) explains that read-write lock types from various crates are used, depending on the mode of operation and the enabled feature flags. However, this is only part of the story. A challenge arises during serialization, where `serde` requires the struct to implement the `Clone` trait. While this is straightforward for plain-mode structs, as deriving the trait works seamlessly, sync/async mode structs may require manual implementation of cloning because none of the `RwLock` implementations support the `Clone` trait.

FieldX implements a solution for this problem by implementing a zero-cost wrapper around the `RwLock` types, `FXRwLock<T>`, which implements support for cloning. Normally its use is activated together with the `serde` feature flag, but can also be requested explicitly by the user with the {{i:`clonable-lock`}} feature flag.[^serde_depends]

[^serde_depends]: The `serde` feature flag actually simply depends on the `clonable-lock`.

But the support comes with a caveat: the `FXRwLock<T>` wrapper has to acquire a read lock on the underlying object to clone it. While it is generally OK in the presence of other read-only locks, it poses a risk of deadlock under certain circumstances like obtaining a long-living write lock and then trying to clone the object deeper in the call stack.

The undesired "surprise" could be aggravated by the use of serialization if one forgets that it utilizes the cloning under the hood. This is where explicit deriving of the `Clone` trait may come to help as a visual reminder about this behavior.

Overall, the practice of modifying data that is being serialized is considered bad, therefore the above-described scenario is more of an "apocalyptic" case. For now, the convenience of automatically enabling `clonable-lock` with the `serde` feature flag is considered an acceptable trade-off that outweighs the risk.

Either way, the current implementation is experimental and the implicit feature flag dependency could be removed in a future minor release.

```admonish info title="Future Plans"
For the moment, FieldX uses `FXRwLock<T>` for all locks when the `clonable-lock` feature flag is enabled. However, it is possible that a future version will introduce better granularity by only defaulting to the wrapper for serializable structs and otherwise requiring something like a `clone` sub-argument of the `lock` argument.
```
