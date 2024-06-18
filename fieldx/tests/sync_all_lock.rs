use fieldx::fxstruct;

#[derive(Default, Debug, PartialEq, Eq)]
struct Bar {
    flag: bool,
}

#[fxstruct(sync, get, lock, builder(into))]
struct Foo {
    text:  String,
    #[fieldx(get(copy), reader)]
    count: usize,
    #[fieldx(lock(off))]
    bare:  Bar,
}

#[test]
fn basics() {
    let obj = Foo::builder()
        .text("hello")
        .count(10usize)
        .bare(Bar::default())
        .build()
        .expect("failed to build");

    assert_eq!(*obj.text(), "hello", "getter of a locked field");
    assert_eq!(obj.count(), 10, "copy-getter of a locked field");
    assert_eq!(*obj.read_count(), 10, "reader of a locked field");
    assert_eq!(obj.bare(), &Bar::default(), "getter of a field with no lock on it");
}
