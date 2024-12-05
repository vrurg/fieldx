use fieldx::fxstruct;

#[fxstruct(builder)]
#[derive(Debug)]
struct FooPlain {
    #[fieldx(optional, get, builder(required, attributes_fn(allow(dead_code))))]
    #[allow(dead_code)]
    v: i32,
}

#[cfg(feature = "sync")]
#[fxstruct(sync, builder)]
#[derive(Debug)]
struct FooSync {
    #[fieldx(optional, get, builder(required, attributes_fn(allow(dead_code))))]
    #[allow(dead_code)]
    v: i32,
}

#[test]
fn plain() {
    let foo = FooPlain::builder().build();
    println!("plain: {:?}", foo);
}

#[cfg(feature = "sync")]
#[test]
fn sync() {
    let foo = FooSync::builder().build();
    println!("sync: {:?}", foo);
}
