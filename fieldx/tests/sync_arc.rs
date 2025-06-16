#![cfg(feature = "sync")]
use fieldx::error::FieldXError;
use fieldx::fxstruct;
use std::sync::Arc;
use std::sync::Weak;

#[fxstruct(sync, rc(vis(pub)), builder)]
struct Bar {
    #[fieldx(get(copy), attributes_fn(allow(dead_code)))]
    id: usize,
}

#[fxstruct(sync, rc, builder)]
struct Foo {
    #[fieldx(lazy, lock, get, get_mut)]
    bar: Arc<Bar>,
}

impl Foo {
    fn build_bar(&self) -> Arc<Bar> {
        Bar::new()
    }
}

#[test]
fn type_check() {
    let foo: Arc<Foo> = Foo::new();
    {
        let bar: fieldx::sync::FXProxyReadGuard<Arc<Bar>> = foo.bar();
        let _bar_copy: Weak<Bar> = bar.myself_downgrade();
        assert_eq!(Arc::weak_count(&bar), 2);
    }
    {
        let mut bar_mut = foo.bar_mut();
        *bar_mut = Bar::builder().id(112233).build().unwrap();
    }
    assert_eq!(foo.bar().id(), 112233);
}

#[test]
fn builder() {
    let foo: Result<Arc<Foo>, FieldXError> = Foo::builder().bar(Bar::new()).build();
    let _foo_copy = Arc::clone(&foo.expect("There was an error producing Foo instance"));
}
