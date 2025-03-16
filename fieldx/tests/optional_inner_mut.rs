use fieldx::fxstruct;

#[fxstruct]
struct FooPlain {
    #[fieldx(inner_mut, optional, get_mut, get, set, clearer, predicate)]
    mutable: String,
}

#[cfg(feature = "sync")]
#[fxstruct(sync)]
struct FooSync {
    #[fieldx(inner_mut, optional, get_mut, get, set, clearer, predicate)]
    mutable: String,
}

#[test]
fn plain() {
    let ns = FooPlain::new();

    assert!(!ns.has_mutable());

    ns.set_mutable("manual".to_string());

    assert_eq!(*ns.mutable(), Some("manual".to_string()));

    assert!(ns.has_mutable());

    let old = ns.clear_mutable();
    assert_eq!(old, Some("manual".to_string()));
    let old = ns.clear_mutable();
    assert_eq!(old, None);
    assert!(!ns.has_mutable());

    *ns.mutable_mut() = Some("via get_mut".to_string());
    assert_eq!(*ns.mutable(), Some("via get_mut".to_string()));
}

#[cfg(feature = "sync")]
#[test]
fn sync() {
    let ns = FooSync::new();

    assert!(!ns.has_mutable());

    ns.set_mutable("manual".to_string());

    assert_eq!(*ns.mutable(), Some("manual".to_string()));

    assert!(ns.has_mutable());

    let old = ns.clear_mutable();
    assert_eq!(old, Some("manual".to_string()));
    let old = ns.clear_mutable();
    assert_eq!(old, None);
    assert!(!ns.has_mutable());

    *ns.mutable_mut() = Some("via get_mut".to_string());
    assert_eq!(*ns.mutable(), Some("via get_mut".to_string()));
}
