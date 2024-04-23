use crate::{
    helper::{
        FXAccessor, FXAccessorMode, FXAttributes, FXBaseHelper, FXFieldBuilder, FXHelper, FXHelperContainer,
        FXHelperKind, FXHelperTrait, FXNestingAttr, FXPubMode, FXSetter, FXWithOrig, FromNestAttr,
    },
    util::{needs_helper, validate_exclusives},
};
use darling::FromField;
use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use std::{cell::OnceCell, ops::Deref};
use syn::{spanned::Spanned, Meta};

#[derive(Debug, FromField, Getters)]
#[getset(get = "pub(crate)")]
#[darling(attributes(fieldx), forward_attrs)]
pub(crate) struct FXFieldReceiver {
    ident: Option<syn::Ident>,
    vis:   syn::Visibility,
    ty:    syn::Type,
    attrs: Vec<syn::Attribute>,

    // Default method attributes for this field.
    attributes_fn: Option<FXAttributes>,
    lazy:          Option<FXHelper>,
    #[darling(rename = "rename")]
    base_name:     Option<String>,
    #[darling(rename = "get")]
    accessor:      Option<FXAccessor>,
    #[darling(rename = "get_mut")]
    accessor_mut:  Option<FXHelper>,
    #[darling(rename = "set")]
    setter:        Option<FXSetter>,
    reader:        Option<FXHelper>,
    writer:        Option<FXHelper>,
    clearer:       Option<FXHelper>,
    predicate:     Option<FXHelper>,

    public:        Option<FXNestingAttr<FXPubMode>>,
    private:       Option<FXWithOrig<bool, syn::Meta>>,
    #[darling(rename = "default")]
    default_value: Option<Meta>,
    builder:       Option<FXFieldBuilder>,
    into:          Option<bool>,
    clone:         Option<bool>,
    copy:          Option<bool>,

    #[darling(skip)]
    #[getset(skip)]
    span: OnceCell<Span>,
}

#[derive(Debug)]
pub(crate) struct FXField(FXFieldReceiver);

impl FromField for FXField {
    fn from_field(field: &syn::Field) -> darling::Result<Self> {
        // eprintln!("@@@ FROM FIELD '{:?}'", if let Some(ref ident) = field.ident { ident.to_string() } else { "<anon>".to_string() });
        let mut fxfield = FXFieldReceiver::from_field(field)?;
        for attr in (&field.attrs).into_iter() {
            // Intercept #[fieldx] form of the attribute and mark the field manually
            if attr.path().is_ident("fieldx") && attr.meta.require_path_only().is_ok() {
                fxfield
                    .mark_implicitly(attr.meta.clone())
                    .map_err(|err| darling::Error::custom(format!("Can't use bare word '{}'", err)).with_span(attr))?;
            }
        }
        if let Err(_) = fxfield.set_span((field as &dyn Spanned).span()) {
            let err = darling::Error::custom("Can't set span for a field receiver object: it's been set already!")
                .with_span(field);
            #[cfg(feature = "diagnostics")]
            let err = err.note("This must not happen normally, please report this error to the author of fieldx");
            return Err(err);
        }
        fxfield.validate()?;
        Ok(Self(fxfield))
    }
}

impl Deref for FXField {
    type Target = FXFieldReceiver;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToTokens for FXField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fxr = &self.0;
        let FXFieldReceiver {
            ident, vis, ty, attrs, ..
        } = fxr;
        tokens.extend(quote_spanned![*fxr.span()=> #( #attrs )* #vis #ident: #ty])
    }
}

impl FXFieldReceiver {
    validate_exclusives! {"visibility" => public, private; "accessor mode" => copy, clone}

    // Generate field-level needs_<helper> methods. The final decision of what's needed and what's not is done by
    // FXFieldCtx.
    needs_helper! {accessor, accessor_mut, builder, clearer, setter, predicate, reader, writer}

    pub fn validate(&self) -> darling::Result<()> {
        self.validate_exclusives().map_err(|err| err.with_span(self.ident()))
    }

    fn flag_set(helper: &Option<FXNestingAttr<impl FXHelperTrait + FromNestAttr>>) -> Option<bool> {
        helper.as_ref().map(|h| h.is_true())
    }

    pub fn public_mode(&self) -> Option<FXPubMode> {
        if self.private.is_some() {
            Some(FXPubMode::Private)
        }
        else {
            self.public.as_ref().map(|pm| (**pm).clone())
        }
    }

    #[inline]
    pub fn is_lazy(&self) -> Option<bool> {
        Self::flag_set(&self.lazy)
    }

    #[inline]
    pub fn is_into(&self) -> Option<bool> {
        self.into
    }

    pub fn is_ignorable(&self) -> bool {
        self.ident.to_token_stream().to_string().starts_with("_")
    }

    pub fn is_setter_into(&self) -> Option<bool> {
        self.setter.as_ref().and_then(|s| s.is_into())
    }

    pub fn is_builder_into(&self) -> Option<bool> {
        self.builder.as_ref().and_then(|b| b.is_into())
    }

    pub fn is_copy(&self) -> Option<bool> {
        self.clone.map(|c| !c).or_else(|| self.copy)
    }

    pub fn is_accessor_copy(&self) -> Option<bool> {
        self.accessor_mode().map(|m| m == FXAccessorMode::Copy)
    }

    pub fn has_default_value(&self) -> bool {
        self.default_value.is_some()
    }

    pub fn accessor_mode(&self) -> Option<FXAccessorMode> {
        self.accessor.as_ref().and_then(|a| a.mode())
    }

    pub fn builder_attributes(&self) -> Option<&FXAttributes> {
        self.builder.as_ref().and_then(|b| b.attributes())
    }

    pub fn builder_fn_attributes(&self) -> Option<&FXAttributes> {
        self.builder.as_ref().and_then(|b| b.attributes_fn())
    }

    fn mark_implicitly(&mut self, orig: Meta) -> Result<(), &str> {
        match self.lazy {
            None => {
                self.lazy = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig.clone())));
                self.clearer = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig.clone())));
                self.predicate = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig)));
            }
            _ => (),
        };
        Ok(())
    }

    #[inline]
    pub fn set_span(&mut self, span: Span) -> Result<(), Span> {
        self.span.set(span)
    }

    #[inline]
    pub fn span(&self) -> &Span {
        self.span.get_or_init(|| Span::call_site())
    }
}

impl FXHelperContainer for FXFieldReceiver {
    fn get_helper(&self, kind: FXHelperKind) -> Option<&dyn FXHelperTrait> {
        match kind {
            // FXHelperKind::Lazy => self.lazy().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Accessor => self.accessor().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::AccesorMut => self.accessor_mut().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Clearer => self.clearer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Predicate => self.predicate().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Reader => self.reader().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Setter => self.setter().as_ref().map(|h| &**h as &dyn FXHelperTrait),
            FXHelperKind::Writer => self.writer().as_ref().map(|h| &**h as &dyn FXHelperTrait),
        }
    }
}
