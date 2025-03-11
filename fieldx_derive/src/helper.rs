pub(crate) use fieldx_aux::FXAccessorMode;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FXHelperKind {
    Accessor,
    AccessorMut,
    Builder,
    Clearer,
    Lazy,
    Predicate,
    Reader,
    Setter,
    Writer,
}

impl ToString for FXHelperKind {
    fn to_string(&self) -> String {
        match self {
            FXHelperKind::Accessor => "accessor",
            FXHelperKind::AccessorMut => "accessor_mut",
            FXHelperKind::Builder => "builder setter",
            FXHelperKind::Clearer => "clearer",
            FXHelperKind::Lazy => "lazy builder",
            FXHelperKind::Predicate => "predicate",
            FXHelperKind::Reader => "reader",
            FXHelperKind::Setter => "setter",
            FXHelperKind::Writer => "writer",
        }
        .to_string()
    }
}

impl FXHelperKind {
    #[inline]
    pub(crate) fn default_prefix(&self) -> &str {
        match self {
            FXHelperKind::AccessorMut => "",
            FXHelperKind::Accessor => "",
            FXHelperKind::Builder => "",
            FXHelperKind::Clearer => "clear_",
            FXHelperKind::Lazy => "build_",
            FXHelperKind::Predicate => "has_",
            FXHelperKind::Reader => "read_",
            FXHelperKind::Setter => "set_",
            FXHelperKind::Writer => "write_",
        }
    }

    #[inline]
    pub(crate) fn default_suffix(&self) -> &str {
        match self {
            FXHelperKind::AccessorMut => "_mut",
            FXHelperKind::Accessor => "",
            FXHelperKind::Builder => "",
            FXHelperKind::Clearer => "",
            FXHelperKind::Lazy => "",
            FXHelperKind::Predicate => "",
            FXHelperKind::Reader => "",
            FXHelperKind::Setter => "",
            FXHelperKind::Writer => "",
        }
    }
}
