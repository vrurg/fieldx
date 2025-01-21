use fieldx::fxstruct;

#[fxstruct(builder(opt_in, into), default(off), get)]
#[derive(Debug)]
struct FooPlain {
    #[fieldx(builder)]
    buildable: String,

    #[fieldx(default("explicit".into()))]
    unbuildable: String,
}

#[cfg(feature = "sync")]
#[fxstruct(sync, builder(opt_in, into), default(off), get)]
#[derive(Debug)]
struct FooSync {
    #[fieldx(builder)]
    buildable: String,

    #[fieldx(default("explicit".into()))]
    unbuildable: String,
}

#[test]
fn plain() {
    let foo = FooPlain::builder().buildable("from builder").build().unwrap();
    assert_eq!(foo.buildable(), "from builder");
    assert_eq!(foo.unbuildable(), "explicit");
}

#[cfg(feature = "sync")]
#[test]
fn nsync() {
    let foo = FooSync::builder().buildable("from builder").build().unwrap();
    assert_eq!(foo.buildable(), "from builder");
    assert_eq!(foo.unbuildable(), "explicit");
}
