#![cfg(feature = "sync")]
use fieldx::fxstruct;

#[derive(PartialEq, Debug)]
struct Bar {
    n: String,
}

#[fxstruct(sync, builder)]
struct Foo {
    #[fieldx(lock, optional, get_mut, predicate, set)]
    b1: Bar,
    #[fieldx(optional, lock(off), get_mut, predicate, set)]
    b2: Bar,
}

#[test]
fn try_b1() {
    let foo = Foo::new();

    assert!(!foo.has_b1(), "no value yet");
    *foo.b1_mut() = Some(Bar {
        n: "foo.b1".to_string(),
    });
    assert!(foo.has_b1(), "now value is set");
    assert_eq!(
        *foo.b1(),
        Some(Bar {
            n: "foo.b1".to_string(),
        }),
        "value itself is correct"
    );

    let _ = foo.set_b1(Bar {
        n: "b1 via setter".into(),
    });
    assert_eq!(
        *foo.b1(),
        Some(Bar {
            n: "b1 via setter".to_string(),
        }),
        "value via a setter"
    );

    let foo = Foo::builder()
        .b1(Bar { n: "manual b1".into() })
        .build()
        .expect("Foo builder failed");
    assert!(foo.has_b1(), "set by the builder");
    assert_eq!(
        *foo.b1(),
        Some(Bar {
            n: "manual b1".to_string(),
        }),
        "value itself is correct"
    );
}

#[test]
fn try_b2() {
    let mut foo = Foo::new();

    assert!(!foo.has_b2(), "no value yet");
    *foo.b2_mut() = Some(Bar {
        n: "foo.b2".to_string(),
    });
    assert!(foo.has_b2(), "now value is set");
    assert_eq!(
        *foo.b2(),
        Some(Bar {
            n: "foo.b2".to_string(),
        }),
        "value itself is correct"
    );

    let _ = foo.set_b2(Bar {
        n: "b2 via setter".into(),
    });
    assert_eq!(
        *foo.b2(),
        Some(Bar {
            n: "b2 via setter".to_string(),
        }),
        "value via a setter"
    );

    let foo = Foo::builder()
        .b2(Bar { n: "manual b2".into() })
        .build()
        .expect("Foo builder failed");
    assert!(foo.has_b2(), "set by the builder");
    assert_eq!(
        *foo.b2(),
        Some(Bar {
            n: "manual b2".to_string(),
        }),
        "value itself is correct"
    );
}
