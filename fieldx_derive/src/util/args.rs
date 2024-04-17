use crate::{
    helper::{
        FXAccessor, FXAccessorMode, FXArgsBuilder, FXAttributes, FXHelper, FXHelperTrait, FXNestingAttr,
        FXPubMode, FXSetter, FXWithOrig,
    },
    util::{needs_helper, validate_exclusives},
};
use darling::{util::Flag, FromMeta};
use getset::Getters;
use proc_macro2::{Span, TokenStream};

#[derive(Debug, FromMeta, Clone, Getters, Default)]
#[darling(and_then = Self::validate)]
#[getset(get = "pub")]
pub(crate) struct FXSArgs {
    sync:    Flag,
    builder: Option<FXArgsBuilder>,
    into:    Option<bool>,
    // Only plays for sync-safe structs
    no_new:  Flag,

    // Field defaults
    lazy:         Option<FXHelper<true>>,
    #[darling(rename = "get")]
    accessor:     Option<FXAccessor<true>>,
    #[darling(rename = "get_mut")]
    accessor_mut: Option<FXHelper<true>>,
    #[darling(rename = "set")]
    setter:       Option<FXSetter<true>>,
    reader:       Option<FXHelper<true>>,
    writer:       Option<FXHelper<true>>,
    clearer:      Option<FXHelper<true>>,
    predicate:    Option<FXHelper<true>>,
    public:       Option<FXNestingAttr<FXPubMode>>,
    private:      Option<FXWithOrig<bool, syn::Meta>>,
    clone:        Option<bool>,
    copy:         Option<bool>,
    // #[darling(skip)]
    // recurs_cnt: RefCell<u32>,
}

impl FXSArgs {
    validate_exclusives!("visibility" => public, private; "accessor mode" => copy, clone);

    // Generate needs_<helper> methods
    needs_helper! {accessor, accessor_mut, setter, reader, writer, clearer, predicate}

    pub fn validate(self) -> Result<Self, darling::Error> {
        self.validate_exclusives()
            .map_err(|err| err.with_span(&Span::call_site()))?;
        Ok(self)
    }

    pub fn is_sync(&self) -> bool {
        self.sync.is_present()
    }

    pub fn is_into(&self) -> Option<bool> {
        self.into
    }

    pub fn is_copy(&self) -> Option<bool> {
        self.clone.map(|c| !c).or_else(|| self.copy)
    }

    pub fn is_accessor_copy(&self) -> Option<bool> {
        self.accessor_mode().map(|m| m == FXAccessorMode::Copy)
    }

    pub fn is_setter_into(&self) -> Option<bool> {
        self.setter.as_ref().and_then(|h| h.is_into())
    }

    pub fn is_builder_into(&self) -> Option<bool> {
        self.builder.as_ref().and_then(|h| h.is_into())
    }

    pub fn needs_new(&self) -> bool {
        !self.no_new.is_present()
    }

    pub fn needs_builder(&self) -> Option<bool> {
        self.builder.as_ref().and(Some(true))
    }

    pub fn is_lazy(&self) -> Option<bool> {
        // *self.recurs_cnt.borrow_mut() += 1;
        // fxtrace!(*self.recurs_cnt.borrow());
        // *self.recurs_cnt.borrow_mut() -= 1;
        // rc
        self.lazy.as_ref().map(|h| h.is_true())
    }

    pub fn accessor_mode(&self) -> Option<FXAccessorMode> {
        self.accessor.as_ref().and_then(|h| h.mode().copied())
    }

    pub fn vis_tok(&self) -> Option<TokenStream> {
        self.public.as_ref().and_then(|p| p.vis_tok().into())
    }

    pub fn builder_attributes(&self) -> Option<&FXAttributes> {
        self.builder.as_ref().and_then(|b| b.attributes().as_ref())
    }

    pub fn builder_impl_attributes(&self) -> Option<&FXAttributes> {
        self.builder.as_ref().and_then(|b| b.attributes_impl().as_ref())
    }
}
