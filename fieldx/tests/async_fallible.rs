#![cfg(feature = "async")]
use fieldx::fxstruct;
use std::sync::Arc;

#[fxstruct(mode(async), fallible(off, error(String)))]
struct Foo<const FAIL: bool = false> {
    #[fieldx(lazy, fallible)]
    ok: i32,

    #[fieldx(lazy, fallible, get(copy), get_mut, set)]
    writable: u32,

    #[fieldx(lazy, fallible, get(clone))]
    shared: Arc<String>,
}

impl<const FAIL: bool> Foo<FAIL> {
    async fn build_ok(&self) -> Result<i32, String> {
        if FAIL {
            Err("will never be there".to_string())
        }
        else {
            Ok(-42)
        }
    }

    async fn build_writable(&self) -> Result<u32, String> {
        if FAIL {
            Err("no value".to_string())
        }
        else {
            Ok(12)
        }
    }

    async fn build_shared(&self) -> Result<Arc<String>, String> {
        if FAIL {
            Err("this is a failed outcome".to_string())
        }
        else {
            Ok(Arc::new("shared".to_string()))
        }
    }
}

#[tokio::test]
async fn fallible_ok() -> Result<(), Box<dyn std::error::Error>> {
    let foo = Foo::<false>::new();

    assert!(*foo.ok().await? == -42);

    assert_eq!(foo.writable().await?, 12);
    *foo.writable_mut().await? = 42;
    assert_eq!(foo.writable().await?, 42);

    assert_eq!(*foo.shared().await?, "shared".to_string());

    Ok(())
}

#[tokio::test]
async fn fallible_error() -> Result<(), Box<dyn std::error::Error>> {
    let foo = Foo::<true>::new();

    assert!(foo.ok().await.is_err());
    assert_eq!(*foo.ok().await.unwrap_err(), String::from("will never be there"));

    assert!(foo.writable().await.is_err());
    assert_eq!(foo.writable().await.unwrap_err(), "no value".to_string());
    foo.set_writable(42).await;
    assert_eq!(foo.writable().await?, 42);

    assert!(foo.shared().await.is_err());
    assert_eq!(
        *foo.shared().await.unwrap_err(),
        String::from("this is a failed outcome")
    );

    Ok(())
}
