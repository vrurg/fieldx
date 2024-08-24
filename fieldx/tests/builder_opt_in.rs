use fieldx::fxstruct;

#[fxstruct(builder(opt_in, into), get)]
#[derive(Debug)]
struct FooNonsync {
    #[fieldx(builder)]
    buildable: String,

    #[fieldx(default("explicit"))]
    unbuildable: String,
}

#[fxstruct(sync, builder(opt_in, into), get)]
#[derive(Debug)]
struct FooSync {
    #[fieldx(builder)]
    buildable: String,

    #[fieldx(default("explicit"))]
    unbuildable: String,
}

#[test]
fn nonsync() {
    let foo = FooNonsync::builder().buildable("from builder").build().unwrap();
    assert_eq!(foo.buildable(), "from builder");
    assert_eq!(foo.unbuildable(), "explicit");
}

#[test]
fn nsync() {
    let foo = FooSync::builder().buildable("from builder").build().unwrap();
    assert_eq!(foo.buildable(), "from builder");
    assert_eq!(foo.unbuildable(), "explicit");
}
