#![allow(clippy::approx_constant)]
use fieldx::fxstruct;

#[fxstruct]
#[derive(Debug)]
struct Plain {
    #[fieldx(lazy, clearer, predicate)]
    foo:    String,
    #[fieldx(lazy, private, predicate, clearer, set, default(13))]
    bar:    i32,
    #[fieldx(default(3.1415926))]
    pub pi: f32,

    // Let's try a charged but not lazy field
    #[fieldx(clearer, predicate, get, set, default(String::from("bazzification")))]
    baz: String,

    // Note the intentional use of a different type for the default value. Without `off`, it would trigger an error!
    #[fieldx(lazy, clearer, rename("piquant"), default(off, "this won't be used"))]
    fubar: String,

    #[fieldx(inner_mut, set, get, get_mut)]
    modifiable: String,
}

impl Plain {
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
fn basic_default() {
    let non_sync = Plain::new();
    assert_eq!(non_sync.pi, 3.1415926, "default value for field pi");
}

#[test]
fn basic_lazies() {
    let mut non_sync = Plain::new();

    assert!(!non_sync.has_foo(), "foo is not initialized yet");
    assert_eq!(non_sync.foo(), "this is foo with bar=13", "both builders are involved");
    assert!(non_sync.has_foo(), "foo has been built");
    assert!(non_sync.has_bar(), "bar has been built");
    assert_eq!(non_sync.clear_bar(), Some(13), "cleared bar, value comes from default");
    assert!(!non_sync.has_bar(), "bar has been cleared");
    assert_eq!(
        non_sync.foo(),
        "this is foo with bar=13",
        "foo remembers old bar value until cleared"
    );
    assert!(
        !non_sync.has_bar(),
        "reading uncleared foo does not trigger bar building"
    );
    assert_eq!(
        non_sync.bar(),
        &42,
        "cleared bar initialized lazily, no default involved"
    );
    non_sync.clear_bar();
    non_sync.clear_foo();
    assert_eq!(non_sync.foo(), &String::from("this is foo with bar=42"));
    assert_eq!(non_sync.set_bar(12), Some(42), "set bar");
    assert!(non_sync.has_bar(), "bar now has a value");
    assert_eq!(
        non_sync.clear_foo(),
        Some(String::from("this is foo with bar=42")),
        "cleared foo"
    );
    assert!(!non_sync.has_foo(), "foo has been cleared");
    assert_eq!(
        non_sync.foo(),
        "this is foo with bar=12",
        "manually set bar is used to rebuild foo"
    );
    assert_eq!(non_sync.piquant(), "щось пікантне", "fubar is built lazily");
    assert_eq!(
        non_sync.clear_piquant(),
        Some(String::from("щось пікантне")),
        "cleared fubar"
    );
}

#[test]
fn basic_nonlazy() {
    let mut non_sync = Plain::new();

    assert!(non_sync.baz().is_some(), "baz is a Some()");
    assert!(non_sync.has_baz(), "baz is set");
    assert_eq!(
        non_sync.baz().as_ref().unwrap(),
        "bazzification",
        "default value for field baz"
    );
    assert_eq!(non_sync.clear_baz(), Some(String::from("bazzification")), "cleared baz");
    assert!(!non_sync.has_baz(), "baz is cleared");
    non_sync.set_baz("new baz".into());
    assert!(non_sync.has_baz(), "baz is set manually");
    assert_eq!(
        non_sync.baz().as_ref().unwrap(),
        "new baz",
        "manually set value for field baz"
    );
}

#[test]
fn inner_mut() {
    let non_sync = Plain::new();

    {
        let old = non_sync.set_modifiable("new value".to_string());
        // There is a Default::default() initially
        assert_eq!(old, "");
        let ms = non_sync.modifiable();
        assert_eq!(*ms, "new value".to_string());
    }

    {
        *non_sync.modifiable_mut() = "modified value".to_string();
        let ms = non_sync.modifiable();
        assert_eq!(*ms, "modified value".to_string());
    }
}
