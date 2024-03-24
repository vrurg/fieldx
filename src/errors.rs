#[derive(Debug)]
pub struct UninitializedFieldError(&'static str);

impl UninitializedFieldError {
    pub fn new(field_name: &'static str) -> Self {
        UninitializedFieldError(field_name)
    }

    pub fn field_name(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for UninitializedFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Field '{}' is not set", self.0)
    }
}

impl std::error::Error for UninitializedFieldError {}

impl From<&'static str> for UninitializedFieldError {
    fn from(field_name: &'static str) -> Self {
        Self::new(field_name)
    }
}
