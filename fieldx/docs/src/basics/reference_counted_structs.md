# Reference Counted Structs {{hi:reference counted}}

Another typical Rust pattern is to use `Rc` (or `Arc`) to allow multiple owners of a struct. There are various possible reasons why one might want to do this, but let us not delve into them here. The point is that sometimes we don't just need an object to be reference counted, but we need or want it to be constructed this way.

```admonish tip title="Example which we promised not to go into.."
OK, let's digress into an example once. Say you need to implement a parent-child relationship between two or more structs. In this case, if there is no pressure from the performance side, the parent object can be reference counted to simplify the task of keeping a reference to it for its children.
```

As always, the way to achieve this pattern is as simple as adding the `rc` argument to the struct declaration:

```rust,ignore
#[fxstruct(rc)]
```

Sync/async and plain modes of operation are supported, resulting in either `Arc` or `Rc` being used as the container type respectively.

The `rc` argument comes with a perk but, at the same time, with a cost.

The cost is an additional implicit field being added to the struct, which holds a [plain](https://doc.rust-lang.org/std/rc/struct.Weak.html) or [sync](https://doc.rust-lang.org/std/sync/struct.Weak.html) `Weak` reference to the object itself.

The perk is two new methods in the implementation: `{{i:myself}}` and `{{i:myself_downgrade}}`. The first one returns a strong reference to the object itself, while the second one returns a weak reference. The latter is useful when you need to pass a reference to the object to some other code that may outlive the object itself, and you want to avoid keeping it alive longer than necessary.

```rust,ignore
{{#include ../../../examples/book_rc.rs:rc_decl}}
```

```admonish warning
Feature flag `sync` is required for this example to compile.
```

Here we use reference counting to perform a self-check in a parallel thread while continuing to serve reader requests.

```admonish
Since the self-reference is weak, the `myself` method must upgrade it first to provide us with a strong reference. Since upgrading gives us an `Option`, it must be unwrapped. This is safe to do in the example code because the object is alive, hence its counter is at least 1. However, within the `drop()` method of the [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) trait, the `myself` method will return `None` since the only possible case when the `Drop` trait is activated is when the reference count reaches zero.
```

Because the `rc` argument results in methods being added to the implementation, it is a helper method. Its literal string argument allows you to change the name of the `myself` method:

```rust,ignore
#[fxstruct(rc("my_own"))]
```
