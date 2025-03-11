use fieldx::fxstruct;

mod my {
    use fieldx::error::FieldXError;
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("Bad field '{0}' value")]
        BadFieldValue(String),

        #[error("builder error: {0}")]
        Builder(#[from] FieldXError),

        #[error("Field '{0}' is not set")]
        UnsetField(String),
    }
}

#[fxstruct(builder(error(my::Error), post_build(check_in)))]
#[derive(Debug)]
struct Plain {
    pub num: f32,
}

impl Plain {
    fn check_in(self) -> Result<Self, my::Error> {
        if self.num < 3.0 {
            return Err(my::Error::BadFieldValue("num".to_string()));
        }

        Ok(self)
    }
}

#[fxstruct(builder(error(my::Error, my::Error::UnsetField), attributes_fn(allow(unused))))]
#[derive(Debug)]
struct Foo {
    #[allow(unused)]
    pub foo: i32,
}

#[test]
fn no_field() {
    let plain = Plain::builder().build();
    assert!(plain.is_err(), "error is returned");
    assert!(
        if let my::Error::Builder(FieldXError::UninitializedField(field)) = plain.unwrap_err() {
            assert_eq!(field, "num".to_string());
            true
        }
        else {
            false
        }
    );
}

#[test]
fn bad_field_value() {
    let plain = Plain::builder().num(2.0).build();
    assert!(plain.is_err(), "error is returned");
    assert!(if let my::Error::BadFieldValue(field) = plain.unwrap_err() {
        assert_eq!(field, "num".to_string());
        true
    }
    else {
        false
    });
}

#[test]
fn custom_no_field() {
    let foo = Foo::builder().build();
    assert!(foo.is_err(), "error is returned");
    assert!(if let my::Error::UnsetField(field) = foo.unwrap_err() {
        assert_eq!(field, "foo".to_string());
        true
    }
    else {
        false
    });
}
