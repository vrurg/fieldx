use darling::FromMeta;

#[derive(Debug, FromMeta, Clone, Copy)]
pub struct FXSArgs {
    sync:        Option<bool>,
    builder:     Option<bool>,
    into:        Option<bool>,
    // Only plays for sync-safe structs
    no_new: Option<bool>,
}

impl Default for FXSArgs {
    fn default() -> Self {
        FXSArgs {
            no_new: Some(false),
            builder:     None,
            sync:        None,
            into:        None,
        }
    }
}

impl FXSArgs {
    pub fn is_sync(&self) -> bool {
        if let Some(ref is_sync) = self.sync {
            *is_sync
        }
        else {
            false
        }
    }

    pub fn needs_new(&self) -> bool {
        if let Some(ref no_new) = self.no_new {
            !*no_new
        }
        else {
            true
        }
    }

    pub fn needs_builder(&self) -> Option<bool> {
        self.builder
    }

    pub fn needs_into(&self) -> Option<bool> {
        self.into
    }
}
