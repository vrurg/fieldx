pub mod props;

use darling::util::Flag;
use darling::FromField;
use darling::FromMeta;
use fieldx_aux::to_tokens_vec;
use fieldx_aux::validate_exclusives;
use fieldx_aux::validate_no_macro_args;
use fieldx_aux::FXAccessor;
use fieldx_aux::FXAttributes;
use fieldx_aux::FXBool;
use fieldx_aux::FXBuilder;
use fieldx_aux::FXDefault;
use fieldx_aux::FXFallible;
use fieldx_aux::FXHelper;
use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
use fieldx_aux::FXSerde;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSetter;
use fieldx_aux::FXString;
use fieldx_aux::FXSynValue;
use fieldx_aux::FXSyncMode;
use getset::Getters;
use once_cell::unsync::OnceCell;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::ToTokens;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::Meta;

#[derive(Debug, FromField, Getters, Clone)]
#[getset(get = "pub")]
#[darling(attributes(fieldx), forward_attrs)]
pub struct FXFieldReceiver {
    #[getset(skip)]
    ident: Option<syn::Ident>,
    vis:   syn::Visibility,
    ty:    syn::Type,
    attrs: Vec<syn::Attribute>,

    skip: Flag,

    #[getset(skip)]
    mode:       Option<FXSynValue<FXSyncMode>>,
    #[getset(skip)]
    #[darling(rename = "sync")]
    mode_sync:  Option<FXBool>,
    #[getset(skip)]
    #[darling(rename = "r#async")]
    mode_async: Option<FXBool>,

    // Default method attributes for this field.
    attributes_fn: Option<FXAttributes>,
    fallible:      Option<FXNestingAttr<FXFallible>>,
    lazy:          Option<FXHelper>,
    #[darling(rename = "rename")]
    #[getset(skip)]
    base_name:     Option<FXString>,
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
    optional:      Option<FXBool>,

    #[darling(rename = "vis")]
    visibility:    Option<FXSynValue<syn::Visibility>>,
    private:       Option<FXBool>,
    #[darling(rename = "default")]
    default_value: Option<FXDefault>,
    builder:       Option<FXBuilder>,
    into:          Option<FXBool>,
    #[getset(get = "pub with_prefix")]
    clone:         Option<FXBool>,
    #[getset(get = "pub with_prefix")]
    copy:          Option<FXBool>,
    lock:          Option<FXBool>,
    inner_mut:     Option<FXBool>,
    serde:         Option<FXSerde>,

    #[darling(skip)]
    #[getset(skip)]
    span: OnceCell<Span>,

    #[darling(skip)]
    fieldx_attrs:     Vec<syn::Attribute>,
    #[darling(skip)]
    fieldx_attr_span: Option<Span>,

    #[darling(skip)]
    #[getset(skip)]
    extra: bool,
}

#[derive(Debug, Clone)]
pub struct FXField(FXFieldReceiver);

impl FXField {
    #[inline]
    pub fn extra(mut self) -> Self {
        self.0.extra = true;
        self
    }

    #[inline(always)]
    pub fn is_extra(&self) -> bool {
        self.0.extra
    }
}

impl FromField for FXField {
    fn from_field(field: &syn::Field) -> darling::Result<Self> {
        let mut fxfield = FXFieldReceiver::from_field(field)?;
        for attr in field.attrs.iter() {
            // Intercept #[fieldx] form of the attribute and mark the field manually
            if attr.path().is_ident("fieldx") {
                fxfield.fieldx_attrs.push(attr.clone());
                if attr.meta.require_path_only().is_ok() {
                    fxfield.fieldx_attr_span = Some(attr.span());
                    fxfield.mark_implicitly(attr.meta.clone()).map_err(|err| {
                        darling::Error::custom(format!("Can't use bare word '{err}'")).with_span(attr)
                    })?;
                }
            }
        }
        if fxfield.set_span((field as &dyn Spanned).span()).is_err() {
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
        let fieldx_attrs = &fxr.fieldx_attrs;
        tokens.extend(quote_spanned![fxr.span()=> #( #attrs )* #( #fieldx_attrs )* #vis #ident: #ty])
    }
}

impl FXFieldReceiver {
    validate_exclusives! {
        "accessor mode": copy; clone;
        "field mode":  lazy; optional;
        "concurrency mode": mode_sync as "sync"; mode_async as "async"; mode;
        "visibility": private; visibility as "vis";
    }

    // Generate field-level needs_<helper> methods. The final decision of what's needed and what's not is done by
    // FXFieldCtx.
    // needs_helper! {accessor, accessor_mut, builder, clearer, setter, predicate, reader, writer}

    pub fn validate(&self) -> darling::Result<()> {
        let mut acc = darling::Error::accumulator();

        if let Err(err) = self.validate_exclusives() {
            acc.push(err);
        }

        #[cfg(feature = "serde")]
        validate_no_macro_args! {
            "field", self, acc:
                serde.shadow_name,
                serde.orig_visibility as visibility,
                serde.private,
        }

        validate_no_macro_args! {
            "field", self, acc:
                builder.prefix,
                builder.method_doc,
                builder.attributes_impl,
                builder.post_build,
                builder.opt_in,
        }

        // XXX Make it a warning when possible.
        if self.fallible().is_set_bool() && !self.lazy().is_set_bool() {
            acc.push(
                darling::Error::custom("Parameter 'fallible' only makes sense when 'lazy' is set too")
                    .with_span(&self.fallible().final_span()),
            );
        }

        #[cfg(not(feature = "sync"))]
        if let Some(err) = crate::util::feature_required("sync", &self.mode_sync) {
            acc.push(err);
        }

        #[cfg(not(feature = "async"))]
        if let Some(err) = crate::util::feature_required("async", &self.mode_async) {
            acc.push(err);
        }

        #[cfg(not(feature = "serde"))]
        if let Some(err) = crate::util::feature_required("serde", &self.serde) {
            acc.push(err);
        }

        acc.finish()?;

        Ok(())
    }

    pub fn ident(&self) -> darling::Result<syn::Ident> {
        self.ident.clone().ok_or_else(|| {
            darling::Error::custom("This is weird, but the field doesn't have an ident!").with_span(&self.span())
        })
    }

    #[inline]
    pub fn has_default_value(&self) -> bool {
        if let Some(ref dv) = self.default_value {
            *dv.is_set()
        }
        else {
            false
        }
    }

    fn mark_implicitly(&mut self, orig: Meta) -> darling::Result<()> {
        if self.lazy.is_none() {
            // self.lazy = Some(FXNestingAttr::new(FXBaseHelper::from(true), Some(orig.clone())));
            self.lazy = Some(FXNestingAttr::from_meta(&syn::parse2::<syn::Meta>(
                quote_spanned! {orig.span()=> lazy},
            )?)?);
            self.clearer = Some(FXNestingAttr::from_meta(&syn::parse2::<syn::Meta>(
                quote_spanned! {orig.span()=> clearer},
            )?)?);
            self.predicate = Some(FXNestingAttr::from_meta(&syn::parse2::<syn::Meta>(
                quote_spanned! {orig.span()=> predicate},
            )?)?);
        };
        Ok(())
    }

    #[inline]
    pub fn set_span(&mut self, span: Span) -> Result<(), Span> {
        self.span.set(span)
    }

    // #[inline]
    // pub fn set_attr_span(&mut self, span: Span) {
    //     self.fieldx_attr_span = Some(span);
    // }

    #[inline]
    pub fn span(&self) -> Span {
        #[allow(clippy::redundant_closure)]
        *self.span.get_or_init(|| Span::call_site())
    }

    pub fn to_arg_tokens(&self) -> Vec<TokenStream> {
        let mut toks = vec![];

        if self.skip.is_present() {
            toks.push(quote_spanned! {self.skip.span()=> skip});
        }

        // Any individual `sync` or `r#async` arguments will be turned into `mode(sync)` or `mode(async)`.
        if let Some(mode) = &self.mode {
            toks.push(mode.to_token_stream())
        }
        else {
            let is_mode_sync = self.mode_sync.is_set();
            let is_mode_async = self.mode_async.is_set();
            if *is_mode_sync {
                toks.push(quote_spanned! {is_mode_sync.final_span()=> mode(sync)});
            }
            else if *is_mode_async {
                toks.push(quote_spanned! {is_mode_async.final_span()=> mode(async)});
            }
        }

        toks.extend(to_tokens_vec!(self:
            base_name,
            builder,
            into,
            default_value,
            attributes_fn,
            accessor,
            accessor_mut,
            setter,
            reader,
            writer,
            clearer,
            predicate,
            clone,
            copy,
            lock,
            inner_mut,
            optional,
            visibility,
            private,
            lazy,
            fallible,
            serde
        ));

        toks
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse2;

    use super::*;

    #[test]
    fn test_attrs() {
        let mode_arg = if cfg!(feature = "async") {
            quote! { r#async }
        }
        else if cfg!(feature = "sync") {
            quote! { sync }
        }
        else {
            quote! {mode(plain)}
        };
        let input = quote! {
            struct Foo {
                #[attr(1)]
                #[fieldx(skip)]
                #[fieldx(#mode_arg, lazy, get_mut)]
                #[fieldx(predicate)]
                foo: u32,
            }
        };
        let foo_struct = parse2::<syn::ItemStruct>(input).unwrap();
        let field: FXField = FXField::from_field(foo_struct.fields.iter().next().unwrap()).unwrap();
        assert_eq!(
            field.to_token_stream().to_string(),
            quote! {
                #[attr(1)]
                #[fieldx(skip)]
                #[fieldx(#mode_arg, lazy, get_mut)]
                #[fieldx(predicate)]
                foo: u32
            }
            .to_string()
        );
    }
}
