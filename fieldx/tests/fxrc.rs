// use fieldx::{fxstruct, FXArc, FXRc};
// use std::{rc::Rc, sync::Arc};

// #[fxstruct]
// #[derive(PartialEq, Debug)]
// struct Foo {
//     v: i32,
// }

// #[test]
// fn basics_rc() {
//     let fxrc = FXRc::<Foo>::new(Foo { v: 42 });

//     assert_eq!(Rc::strong_count(&fxrc), 1, "refcount initially is 1");

//     let foo_rc = Rc::clone(&fxrc);
//     assert_eq!(Rc::strong_count(&fxrc), 2, "refcount is 2 after cloning");

//     let rf = &**fxrc;
//     let rr = &*foo_rc;
//     assert_eq!(*rf, *rr);
// }

// #[test]
// fn basics_arc() {
//     let fxrc = FXArc::<Foo>::new(Foo { v: 42 });

//     assert_eq!(Arc::strong_count(&fxrc), 1, "refcount initially is 1");

//     let foo_rc = Arc::clone(&fxrc);
//     assert_eq!(Arc::strong_count(&fxrc), 2, "refcount is 2 after cloning");

//     let rf = &**fxrc;
//     let rr = &*foo_rc;
//     assert_eq!(*rf, *rr);
// }
