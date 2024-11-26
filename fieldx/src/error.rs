use thiserror::Error;

/// Standard error type that is used by default.
#[derive(Error, Debug, Clone)]
pub enum FieldXError {
    /// This variant indicates that a required field hasn't been set with builder object.
    #[error("Field '{0}' is not set")]
    UninitializedField(String),
    /// A post-build method may report a problem with this variant.
    #[error("Post-build task failed: {0}")]
    PostBuild(String),
}

impl FieldXError {
    #[doc(hidden)]
    pub fn uninitialized_field(field_name: String) -> FieldXError {
        FieldXError::UninitializedField(field_name)
    }

    /// A convenience method for post-build method.
    pub fn post_build<S: ToString>(msg: S) -> FieldXError {
        FieldXError::PostBuild(msg.to_string())
    }
}
