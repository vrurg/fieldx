mod no_new {

    use fieldx::fxstruct;
    #[fxstruct(new(off))]
    struct Foo {
        #[fieldx(get(clone), set, into)]
        bar: String,
    }

    impl Foo {
        fn new(bar: String) -> Self {
            Self { bar }
        }
    }

    #[test]
    fn test_foo_creation() {
        let foo = Foo::new("test".to_string());
        assert_eq!(foo.bar, "test");
    }
}

mod new_name {
    use fieldx::fxstruct;
    #[fxstruct(new("_my_new", private))]
    struct Foo {
        #[fieldx(get(clone), set, default("from default".into()))]
        bar: String,
    }

    impl Foo {
        fn new() -> Self {
            Self::_my_new()
        }
    }

    #[test]
    fn test_foo_creation() {
        let foo = Foo::new();
        assert_eq!(foo.bar, "from default");
    }
}
