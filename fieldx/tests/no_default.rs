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

#[fxstruct(get(vis(pub)), builder, new(off), default(off))]
struct Plain {
    #[fieldx(set)]
    bar: Bar,

    #[fieldx(lazy, clearer, builder(off))]
    b2: Bar,

    #[fieldx(get(clone), set, into)]
    b3: Bar,
}

impl Plain {
    fn new(bar: Bar) -> Self {
        Self {
            bar,
            b2: (Bar {
                note: "from new".to_string(),
            })
            .into(),
            b3: Bar {
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

#[cfg(feature = "sync")]
#[fxstruct(sync, get(vis(pub)), builder, new(off), default(off))]
struct IsSync {
    #[fieldx(set)]
    bar: Bar,

    #[fieldx(lock, clearer)]
    b2: Bar,
}

#[cfg(feature = "sync")]
impl IsSync {
    fn new(bar: Bar) -> Self {
        Self {
            bar,
            b2: Default::default(),
        }
    }
}

#[test]
fn plain() {
    let mut plain = Plain::new(Bar { note: "manual".into() });
    assert_eq!(plain.bar().note, "manual".to_string());
    assert_eq!(plain.b2().note, "from new".to_string());
    plain.clear_b2();
    assert_eq!(plain.b2().note, "from build".to_string());

    plain.set_b3("set+into");
    assert_eq!(plain.b3().note, "set+into".to_string());

    let mut plain = Plain::builder()
        .bar(Bar {
            note: "from builder".into(),
        })
        .b3("builder+into")
        .build()
        .expect("Plain::builder() failed");

    assert_eq!(plain.bar().note, "from builder".to_string());
    assert_eq!(plain.b3().note, "builder+into".to_string());

    plain.set_bar(Bar {
        note: "manual".to_string(),
    });
    assert_eq!(plain.bar().note, "manual".to_string());
}

#[cfg(feature = "sync")]
#[test]
fn sync() {
    let sync = IsSync::new(Bar { note: "manual".into() });
    assert_eq!(sync.bar().note, "manual".to_string());
    assert!(sync.b2().is_none());

    let mut sync = IsSync::builder()
        .bar(Bar {
            note: "from builder".into(),
        })
        .b2(Bar {
            note: "from builder 2".into(),
        })
        .build()
        .expect("IsSync::builder() failed");

    assert_eq!(sync.bar().note, "from builder".to_string());
    assert_eq!(sync.b2().as_ref().unwrap().note, "from builder 2".to_string());
    sync.clear_b2();
    assert!(sync.b2().is_none());

    sync.set_bar(Bar {
        note: "manual".to_string(),
    });
    assert_eq!(sync.bar().note, "manual".to_string());
}
