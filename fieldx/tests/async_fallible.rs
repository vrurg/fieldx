use fieldx::fxstruct;

#[fxstruct(mode(async), fallible(off, error(String)))]
struct Foo {
    #[fieldx(lazy, fallible)]
    never_ok: i32,

    #[fieldx(lazy, fallible, get_mut)]
    writable: u32,
}

impl Foo {
    async fn build_never_ok(&self) -> Result<i32, String> {
        Err("will never be there".to_string())
    }

    async fn build_writable(&self) -> Result<u32, String> {
        Ok(12)
    }
}

#[tokio::test]
async fn fallible() -> Result<(), Box<dyn std::error::Error>> {
    let foo = Foo::new();
    assert!(foo.never_ok().await.is_err());
    assert_eq!(*foo.never_ok().await.unwrap_err(), String::from("will never be there"));

    assert_eq!(*foo.writable().await?, 12);
    *foo.writable_mut().await? = 42;
    assert_eq!(*foo.writable().await?, 42);

    Ok(())
}
