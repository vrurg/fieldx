use fieldx::fxstruct;

#[fxstruct(builder)]
#[derive(Debug)]
struct NonSync {
    #[fieldx(lazy, clearer, predicate)]
    foo:    String,
    #[fieldx(lazy, private, predicate, clearer, setter)]
    bar:    i32,
    #[fieldx(default = 3.1415926)]
    pub pi: f32,

    // Let's try a charged but not lazy field
    #[fieldx(clearer, predicate, setter, default = "bazzification")]
    baz: String,

    #[fieldx(lazy, clearer, rename = "piquant")]
    fubar: String,
}

impl NonSync {
    fn build_foo(&self) -> String {
        format!("this is foo with bar={}", self.bar()).to_string()
    }

    fn build_bar(&self) -> i32 {
        42
    }

    fn build_piquant(&self) -> String {
        "щось пікантне".to_string()
    }
}

#[test]
fn basic() {
    let mut nonsync = NonSync::builder()
        .fubar("as banal as it gets".into())
        .pi(-1.2)
        .build().expect("NonSync instance");

    println!("fubar(piquant) clearing   : {}", nonsync.clear_piquant().unwrap());
    println!("fubar(piquant) after clear: {}", nonsync.piquant());
    println!("pi: {}", nonsync.pi);
}
