use fieldx::fxstruct;

#[fxstruct]
#[derive(Clone, PartialEq, Debug)]
struct Bar {
    note: String,
}

#[fxstruct(builder(vis(pub(crate))))]
pub struct Foo {
    #[fieldx(predicate, get(private, as_ref))]
    b1: Bar,
}

impl Foo {}

#[test]
fn accessors() {
    let foo = Foo::builder().b1(Bar::new()).build().expect("Foo builder failed");

    let b1 = foo.b1();

    assert_eq!(b1, Some(&Bar::new()));
}
