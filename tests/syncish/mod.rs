use fieldx::fxstruct;
use parking_lot::Mutex;

#[fxstruct(sync)]
#[derive(Debug)]
pub struct Foo {
    #[fieldx(lazy, clearer, predicate)]
    foo:    String,
    #[fieldx(lazy, reader, predicate, clearer, set)]
    bar:    i32,
    #[fieldx(default = 3.1415926)]
    pub pi: f32,

    // Let's try a charged but not lazy field
    #[fieldx(clearer, predicate, set, default = "bazzification")]
    baz: String,

    #[fieldx(lazy, clearer)]
    fubar: String,

    bar_builds: Mutex<i32>,
    next_bar:   Mutex<Option<i32>>,
}

impl Foo {
    fn build_foo(&self) -> String {
        format!("Foo with bar={:?}", *self.read_bar()).to_string()
    }

    fn build_bar(&self) -> i32 {
        *self.bar_builds.lock() += 1;
        if let Some(nb) = *self.next_bar.lock() {
            nb
        }
        else {
            42
        }
    }

    fn build_fubar(&self) -> String {
        "аби було".to_string()
    }

    pub fn set_next_bar(&self, next_val: i32) {
        *self.next_bar.lock() = Some(next_val);
    }

    pub fn bar_builds(&self) -> i32 {
        *self.bar_builds.lock()
    }
}
