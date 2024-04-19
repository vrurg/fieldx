use thiserror::Error;

#[derive(Error, Debug)]
pub enum FieldXError {
    #[error("Field '{0}' is not set")]
    UninitializedField(String),
}

impl FieldXError {
    pub fn uninitialized_field(field_name: String) -> FieldXError {
        FieldXError::UninitializedField(field_name)
    }
}
