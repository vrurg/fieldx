use crate::helper::{FXHelper, FXHelperKind};
use darling::FromField;
use getset::Getters;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use std::{cell::OnceCell, ops::Deref};
use syn::{spanned::Spanned, Meta};

#[derive(Debug, FromField, Getters)]
#[getset(get = "pub")]
#[darling(attributes(fieldx), forward_attrs)]
pub(crate) struct FXFieldReceiver {
    ident: Option<syn::Ident>,
    vis:   syn::Visibility,
    ty:    syn::Type,
    attrs: Vec<syn::Attribute>,

    lazy:         Option<FXHelper>,
    #[darling(rename = "rename")]
    base_name:    Option<String>,
    #[darling(rename = "get")]
    accessor:     Option<FXHelper>,
    #[darling(rename = "get_mut")]
    accessor_mut: Option<FXHelper>,
    #[darling(rename = "set")]
    setter:       Option<FXHelper>,
    #[darling(default = "FXHelper::truthy")]
    reader:       Option<FXHelper>,
    #[darling(default = "FXHelper::truthy")]
    writer:       Option<FXHelper>,
    clearer:      Option<FXHelper>,
    predicate:    Option<FXHelper>,
    private:      Option<bool>,
    default:      Option<Meta>,
    builder:      Option<FXHelper>,
    into:         Option<bool>,
    copy:         Option<bool>,

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
                fxfield.mark_implicitly(attr.meta.clone());
            }
        }
        if let Err(_) = fxfield.set_span((field as &dyn Spanned).span()) {
            return Err(
                darling::Error::custom("Can't set span for a field receiver object: it's been set already!")
                    .with_span(field)
                    .note("This must not happen normally, please report this error to the author of fieldx"),
            );
        }
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
    fn flag_set(helper: &Option<FXHelper>) -> bool {
        if let Some(ref helper) = helper {
            match helper.value() {
                FXHelperKind::Flag(flag) => *flag,
                FXHelperKind::Name(mname) => !mname.is_empty(),
            }
        }
        else {
            false
        }
    }

    pub fn needs_accessor(&self, is_sync: bool) -> bool {
        if self.accessor.is_some() {
            Self::flag_set(&self.accessor)
        }
        else if is_sync {
            // A sync struct is better off using reader instead.
            false
        }
        else {
            Self::flag_set(&self.clearer) || Self::flag_set(&self.predicate) || Self::flag_set(&self.lazy)
        }
    }

    pub fn needs_accessor_mut(&self) -> bool {
        Self::flag_set(&self.accessor_mut)
    }

    #[inline]
    pub fn needs_builder(&self) -> Option<bool> {
        if self.builder.is_some() {
            Some(Self::flag_set(&self.builder))
        }
        else {
            None
        }
    }

    #[inline]
    pub fn needs_reader(&self) -> bool {
        Self::flag_set(&self.reader)
    }

    #[inline]
    pub fn needs_writer(&self) -> bool {
        Self::flag_set(&self.reader) && (self.is_lazy() || self.is_optional())
    }

    #[inline]
    pub fn needs_setter(&self) -> bool {
        Self::flag_set(&self.setter)
    }

    #[inline]
    pub fn needs_clearer(&self) -> bool {
        Self::flag_set(&self.clearer) && (self.is_lazy() || self.is_optional())
    }

    #[inline]
    pub fn needs_predicate(&self) -> bool {
        Self::flag_set(&self.predicate) && (self.is_lazy() || self.is_optional())
    }

    #[inline]
    pub fn needs_into(&self) -> Option<bool> {
        self.into
    }

    #[inline]
    pub fn is_lazy(&self) -> bool {
        Self::flag_set(&self.lazy)
    }

    #[inline]
    pub fn is_optional(&self) -> bool {
        !Self::flag_set(&self.lazy) && (Self::flag_set(&self.clearer) || Self::flag_set(&self.predicate))
    }

    pub fn is_pub(&self) -> bool {
        !self.private.unwrap_or(false)
    }

    pub fn is_into(&self) -> bool {
        self.into.unwrap_or(false)
    }

    pub fn is_copy(&self) -> bool {
        self.copy.unwrap_or(false)
    }

    pub fn is_ignorable(&self) -> bool {
        self.ident.to_token_stream().to_string().starts_with("_")
    }

    pub fn has_default(&self) -> bool {
        self.default.is_some()
    }

    fn mark_implicitly(&mut self, orig: Meta) {
        match self.lazy {
            None => {
                self.lazy = Some(FXHelper::new(FXHelperKind::Flag(true), orig.clone()));
                self.clearer = Some(FXHelper::new(FXHelperKind::Flag(true), orig.clone()));
                self.predicate = Some(FXHelper::new(FXHelperKind::Flag(true), orig));
            }
            _ => (),
        };
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
