#![cfg(feature = "sync")]
use fieldx::fxstruct;
use parking_lot::Mutex;

#[fxstruct(sync)]
#[derive(Debug)]
pub struct Foo {
    #[fieldx(lazy, writer, clearer, predicate)]
    foo: String,
    #[fieldx(lazy, writer, predicate, clearer, set)]
    bar: i32,

    // Let's try a charged but not lazy field
    #[fieldx(reader, writer, get(off), clearer, predicate, set, default("bazzification".to_string()))]
    baz: String,

    #[fieldx(lazy, clearer, default("fufubarik!".to_string()))]
    fubar: String,

    bar_builds: Mutex<i32>,
    next_bar:   Mutex<Option<i32>>,
}

impl Foo {
    fn build_foo(&self) -> String {
        format!("Foo with bar={:?}", *self.bar()).to_string()
    }

    fn build_bar(&self) -> i32 {
        *self.bar_builds.lock() += 1;
        self.next_bar.lock().unwrap_or(42)
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
