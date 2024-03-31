use fieldx::fxstruct;

#[fxstruct]
#[derive(Debug)]
struct NonSync<T>
where
    T: std::fmt::Debug + Clone + Send + Sync + Default,
{
    #[fieldx(lazy, clearer, predicate)]
    foo:    String,
    #[fieldx(lazy, private, predicate, clearer, set, copy)]
    bar:    i32,
    #[fieldx(default = 3.1415926)]
    pub pi: f32,

    // Let's try a charged but not lazy field
    #[fieldx(clearer, predicate, set, default = "bazzification")]
    baz: String,

    #[fieldx(lazy, clearer, rename = "piquant")]
    fubar: String,

    #[fieldx(lazy, clearer, predicate)]
    maybe: Option<T>,
}

impl<T> NonSync<T>
where
    T: std::fmt::Debug + Clone + Send + Sync + Default,
{
    fn build_foo(&self) -> String {
        format!("this is foo with bar={}", self.bar()).to_string()
    }

    fn build_bar(&self) -> i32 {
        42
    }

    fn build_piquant(&self) -> String {
        "щось пікантне".to_string()
    }

    fn build_maybe(&self) -> Option<T> {
        Some(T::default())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Dummy;


#[test]
fn basic_default() {
    let non_sync = NonSync::<Dummy>::new();
    assert_eq!(non_sync.pi, 3.1415926, "default value for field pi");
}

#[test]
fn basic_lazies() {
    let mut non_sync = NonSync::<Dummy>::new();

    assert!(!non_sync.has_foo(), "foo is not initialized yet");
    assert_eq!(non_sync.foo(), "this is foo with bar=42", "both builders are involved");
    assert!(non_sync.has_foo(), "foo has been built");
    assert!(non_sync.has_bar(), "bar has been built");
    assert_eq!(non_sync.bar(), 42, "bar accessor is using Copy trait");
    assert_eq!(non_sync.clear_bar(), Some(42), "cleared bar");
    assert!(!non_sync.has_bar(), "bar has been cleared");
    assert_eq!(non_sync.foo(), "this is foo with bar=42", "foo ");
    assert!(
        !non_sync.has_bar(),
        "reading uncleared foo does not trigger bar building"
    );
    assert_eq!(non_sync.set_bar(12), None, "set bar");
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
    let mut non_sync = NonSync::<Dummy>::new();

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
fn optional() {
    let mut non_sync = NonSync::<Dummy>::new();

    assert_eq!(non_sync.maybe(), &Some(Dummy::default()), "an optional field gets initialized");
    assert_eq!(non_sync.clear_maybe(), Some(Some(Dummy::default())), "optional field clear");
    assert!(!non_sync.has_maybe(), "optional field is empty after clearing");
}