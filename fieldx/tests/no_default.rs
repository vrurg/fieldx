use fieldx::fxstruct;

#[derive(Clone, Debug)]
struct Bar {
    note: String,
}

impl From<&str> for Bar {
    fn from(note: &str) -> Self {
        Self { note: note.to_string() }
    }
}

#[fxstruct(get(public), builder, default(off))]
struct NonSync {
    #[fieldx(set)]
    bar: Bar,

    #[fieldx(lazy, clearer)]
    b2: Bar,

    #[fieldx(get(clone), set, into)]
    b3: Bar,
}

impl NonSync {
    fn new(bar: Bar) -> Self {
        Self {
            bar: bar.into(),
            b2:  (Bar {
                note: "from new".to_string(),
            })
            .into(),
            b3:  Bar {
                note: "b3 from new".to_string(),
            },
        }
    }

    fn build_b2(&self) -> Bar {
        Bar {
            note: "from build".to_string(),
        }
    }
}

#[fxstruct(sync, get(public), builder, no_new)]
struct Sync {
    #[fieldx(set)]
    bar: Bar,

    #[fieldx(clearer)]
    b2: Bar,
}

impl Sync {
    fn new(bar: Bar) -> Self {
        Self {
            bar: bar.into(),
            b2:  Default::default(),
        }
    }
}

#[test]
fn nonsync() {
    let mut nonsync = NonSync::new(Bar { note: "manual".into() });
    assert_eq!(nonsync.bar().note, "manual".to_string());
    assert_eq!(nonsync.b2().note, "from new".to_string());
    nonsync.clear_b2();
    assert_eq!(nonsync.b2().note, "from build".to_string());

    nonsync.set_b3("set+into");
    assert_eq!(nonsync.b3().note, "set+into".to_string());

    let mut nonsync = NonSync::builder()
        .bar(Bar {
            note: "from builder".into(),
        })
        .b3("builder+into")
        .build()
        .expect("NonSync::builder() failed");

    assert_eq!(nonsync.bar().note, "from builder".to_string());
    assert_eq!(nonsync.b3().note, "builder+into".to_string());

    nonsync.set_bar(Bar {
        note: "manual".to_string(),
    });
    assert_eq!(nonsync.bar().note, "manual".to_string());
}

#[test]
fn sync() {
    let sync = Sync::new(Bar { note: "manual".into() });
    assert_eq!(sync.bar().note, "manual".to_string());
    assert!(sync.b2().is_none());

    let mut sync = Sync::builder()
        .bar(Bar {
            note: "from builder".into(),
        })
        .b2(Bar {
            note: "from builder 2".into(),
        })
        .build()
        .expect("Sync::builder() failed");

    assert_eq!(sync.bar().note, "from builder".to_string());
    assert_eq!(sync.b2().as_ref().unwrap().note, "from builder 2".to_string());
    sync.clear_b2();
    assert!(sync.b2().is_none());

    sync.set_bar(Bar {
        note: "manual".to_string(),
    });
    assert_eq!(sync.bar().note, "manual".to_string());
}
