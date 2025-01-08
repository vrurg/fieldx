use fieldx::fxstruct;
#[fxstruct(get_mut)]
struct Foo {
    #[fieldx(lazy, predicate)]
    lazish: String,

    #[fieldx(predicate)]
    std: String,

    #[fieldx(lazy, inner_mut)]
    mutable: String,
}

impl Foo {
    fn build_lazish(&self) -> String {
        "got from the builder".to_string()
    }

    fn build_mutable(&self) -> String {
        "mutable".to_string()
    }
}

#[test]
fn mutables() {
    let mut foo = Foo::new();

    assert!(!foo.has_lazish(), "lazish is not initialized yet");
    assert_eq!(
        foo.lazish_mut(),
        "got from the builder",
        "lazish mutable accessor returns built value"
    );
    assert!(foo.has_lazish(), "lazish has been marked initialized");
    *foo.lazish_mut() = "from the user".to_string();
    assert_eq!(foo.lazish(), "from the user", "lazish is set manually");

    eprintln!("do we have std? {} // {:?}", foo.has_std(), foo.std());
    assert!(!foo.has_std(), "non-lazy field isn't set");
    *foo.std_mut() = Some("manually set".to_string());
    assert_eq!(
        *foo.std(),
        Some("manually set".to_string()),
        "manual assignment into mutable accessor"
    );
    assert!(foo.has_std(), "manual assignment set predicate to true");

    let foo_ro = Foo::new();
    assert_eq!(*foo_ro.mutable(), "mutable");
    foo_ro.mutable_mut().push_str(" is mutable");
}
