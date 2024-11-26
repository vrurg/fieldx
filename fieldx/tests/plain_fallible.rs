use fieldx::fxstruct;
use std::rc::Rc;

#[fxstruct(mode(plain), fallible(off, error(String)))]
struct Foo<const FAIL: bool = false> {
    #[fieldx(lazy, fallible)]
    ok: i32,

    #[fieldx(lazy, fallible, get(copy), get_mut, set)]
    writable: u32,

    #[fieldx(lazy, fallible, get(clone))]
    shared: Rc<String>,
}

impl<const FAIL: bool> Foo<FAIL> {
    fn build_ok(&self) -> Result<i32, String> {
        if FAIL {
            Err("will never be there".to_string())
        }
        else {
            Ok(-42)
        }
    }

    fn build_writable(&self) -> Result<u32, String> {
        if FAIL {
            Err("no value".to_string())
        }
        else {
            Ok(12)
        }
    }

    fn build_shared(&self) -> Result<Rc<String>, String> {
        if FAIL {
            Err("this is a failed outcome".to_string())
        }
        else {
            Ok(Rc::new("shared".to_string()))
        }
    }
}

#[test]
fn fallible_ok() -> Result<(), Box<dyn std::error::Error>> {
    let mut foo = Foo::<false>::new();

    assert!(*foo.ok()? == -42);

    assert_eq!(foo.writable()?, 12);
    *foo.writable_mut()? = 42;
    assert_eq!(foo.writable()?, 42);

    assert_eq!(*foo.shared()?, "shared".to_string());

    Ok(())
}

#[test]
fn fallible_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut foo = Foo::<true>::new();

    assert!(foo.ok().is_err());
    assert_eq!(*foo.ok().unwrap_err(), String::from("will never be there"));

    assert!(foo.writable().is_err());
    assert_eq!(foo.writable().unwrap_err(), "no value".to_string());
    foo.set_writable(42);
    assert_eq!(foo.writable()?, 42);

    assert!(foo.shared().is_err());
    assert_eq!(*foo.shared().unwrap_err(), String::from("this is a failed outcome"));

    Ok(())
}
