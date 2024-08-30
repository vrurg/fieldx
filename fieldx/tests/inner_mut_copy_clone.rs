// Make sure inner_mut doesn't break copy and clone getters

use fieldx::fxstruct;

#[fxstruct(sync, builder)]
struct Foo {
    #[fieldx(inner_mut, get(copy))]
    m1: u32,

    #[fieldx(inner_mut, get(clone))]
    m2: String,
}

#[test]
fn basic() {
    let foo = Foo::builder()
        .m1(42)
        .m2("from builder".to_string())
        .build()
        .expect("Builder failed");

    assert_eq!(foo.m1(), 42);
    assert_eq!(foo.m2(), "from builder".to_string());
}
