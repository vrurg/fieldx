use fieldx::fxstruct;

#[fxstruct(sync, lazy, get)]
struct FooS {
    #[fieldx(attributes_fn(allow(dead_code)))]
    bar:        String,
    #[fieldx(skip, default(3.1415926))]
    bare_field: f64,
}

#[fxstruct(lazy, get)]
struct FooN {
    #[fieldx(attributes_fn(allow(dead_code)))]
    bar:        String,
    // Only the `default` to be respected here
    #[fieldx(skip, default(321.654), lazy, get, set, predicate, clearer)]
    bare_field: f64,
}

impl FooS {
    fn build_bar(&self) -> String {
        "test sync".to_string()
    }
}

impl FooN {
    fn build_bar(&self) -> String {
        "test nonsync".to_string()
    }
}

#[test]
fn basic() {
    let foo = FooS::new();
    assert_eq!(foo.bar(), "test sync");
    assert_eq!(foo.bare_field, 3.1415926);

    let foo = FooN::new();
    assert_eq!(foo.bar(), "test nonsync");
    assert_eq!(foo.bare_field, 321.654);
}
