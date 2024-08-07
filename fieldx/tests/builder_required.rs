use fieldx::fxstruct;

#[fxstruct(builder)]
#[derive(Debug)]
struct FooNonsync {
    #[fieldx(optional, get, builder(required, attributes_fn(allow(dead_code))))]
    #[allow(dead_code)]
    v: i32,
}

#[fxstruct(sync, builder)]
#[derive(Debug)]
struct FooSync {
    #[fieldx(optional, get, builder(required, attributes_fn(allow(dead_code))))]
    #[allow(dead_code)]
    v: i32,
}

#[test]
fn nonsync() {
    let foo = FooNonsync::builder().build();
    println!("nonsync: {:?}", foo);
}

#[test]
fn sync() {
    let foo = FooSync::builder().build();
    println!("sync: {:?}", foo);
}
