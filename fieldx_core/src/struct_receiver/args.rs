pub mod props;

use darling::FromMeta;
use fieldx_aux::join_token_list;
use fieldx_aux::to_tokens_vec;
use fieldx_aux::validate_exclusives;
use fieldx_aux::validate_no_macro_args;
use fieldx_aux::FXAccessor;
use fieldx_aux::FXAttributes;
use fieldx_aux::FXBool;
use fieldx_aux::FXBuilder;
use fieldx_aux::FXFallible;
use fieldx_aux::FXHelper;
use fieldx_aux::FXHelperTrait;
use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
use fieldx_aux::FXSerde;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSetter;
use fieldx_aux::FXSynValue;
use fieldx_aux::FXSyncMode;
use getset::Getters;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;

#[derive(Debug, FromMeta, Clone, Getters, Default)]
#[darling(and_then = Self::validate)]
#[getset(get = "pub")]
pub struct FXStructArgs {
    #[getset(skip)]
    mode:       Option<FXSynValue<FXSyncMode>>,
    #[getset(skip)]
    #[darling(rename = "sync")]
    mode_sync:  Option<FXBool>,
    #[getset(skip)]
    #[darling(rename = "r#async")]
    mode_async: Option<FXBool>,

    builder: Option<FXBuilder<true>>,
    into:    Option<FXBool>,

    no_new:  Option<FXBool>,
    new:     Option<FXHelper>,
    default: Option<FXBool>,
    // Produce reference counted object; i.e. Rc<Self> or Arc<Self>.
    rc:      Option<FXHelper>,

    attributes:      Option<FXAttributes>,
    attributes_fn:   Option<FXAttributes>,
    attributes_impl: Option<FXAttributes>,

    // Field defaults
    fallible:     Option<FXNestingAttr<FXFallible>>,
    lazy:         Option<FXHelper>,
    #[darling(rename = "get")]
    accessor:     Option<FXAccessor>,
    #[darling(rename = "get_mut")]
    accessor_mut: Option<FXHelper>,
    #[darling(rename = "set")]
    setter:       Option<FXSetter>,
    reader:       Option<FXHelper>,
    writer:       Option<FXHelper>,
    clearer:      Option<FXHelper>,
    predicate:    Option<FXHelper>,
    optional:     Option<FXBool>,
    #[darling(rename = "vis")]
    visibility:   Option<FXSynValue<syn::Visibility>>,
    private:      Option<FXBool>,
    #[getset(get = "pub with_prefix")]
    clone:        Option<FXBool>,
    #[getset(get = "pub with_prefix")]
    copy:         Option<FXBool>,
    lock:         Option<FXBool>,
    inner_mut:    Option<FXBool>,
    serde:        Option<FXSerde>,
}

impl FXStructArgs {
    validate_exclusives!(
        "accessor mode": copy; clone;
        "concurrency mode": mode_sync as "sync", mode_async as "r#async"; mode;
        "field mode": lazy; optional;
        "serde/ref.counting": serde; rc;
        "visibility": private; visibility as "vis";
    );

    #[inline]
    pub fn validate(self) -> Result<Self, darling::Error> {
        let mut acc = darling::Error::accumulator();

        if let Err(err) = self.validate_exclusives() {
            acc.push(err);
        }

        validate_no_macro_args! {
            "struct", self, acc:
                accessor as get.doc,
                accessor_mut as get_mut.doc,
                clearer.doc,
                predicate.doc,
                reader.doc,
                setter.doc,
                writer.doc,
                lazy.doc
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

        Ok(self)
    }

    pub fn to_arg_tokens(&self) -> Vec<TokenStream> {
        let mut toks = vec![];

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

        if let Some(new) = &self.new {
            toks.push(new.to_token_stream());
        }
        else {
            let is_no_new = self.no_new.is_set();
            if *is_no_new {
                toks.push(quote_spanned! {is_no_new.final_span()=> new(off)});
            }
        }

        toks.extend(to_tokens_vec!(self:
            builder,
            into, default, rc,
            attributes, attributes_fn, attributes_impl,
            fallible, lazy, accessor, accessor_mut,
            setter, reader, writer, clearer,
            predicate, optional, visibility,
            private, clone, copy, lock,
            inner_mut, serde
        ));

        toks
    }
}

impl ToTokens for FXStructArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let toks = self.to_arg_tokens();
        let attr_args = join_token_list!(toks);
        tokens.extend(quote! {fxstruct(#attr_args)});
    }
}

#[cfg(test)]
mod tests {
    use darling::FromMeta;
    use darling::ToTokens;
    use quote::quote;

    use crate::struct_receiver::args::FXStructArgs;

    #[test]
    fn test_roundtrip() {
        let input: syn::Meta = syn::parse2(quote! {
            fxstruct(
                mode(async),
                new(off),
                builder(opt_in, "TestBuilder"),
                into,
                default(off),
                rc,
                attributes( third_party(1,2,3) ),
                attributes_fn( deny(unused) ),
                attributes_impl( deny(unused) ),
                fallible(off, error(crate::error::MyError)),
                lazy(off),
                get("get_"),
                get_mut,
                set,
                reader(off),
                writer(off),
                clearer(off),
                predicate(off),
                optional,
                vis(pub(crate)),
                private(off),
                clone(off),
                copy(off),
                lock(off),
                inner_mut(off),
            )
        })
        .unwrap();

        let args = FXStructArgs::from_meta(&input).unwrap();

        let expected = quote! {
            fxstruct(
                mode(async),
                new(off),
                builder(name("TestBuilder"), opt_in()),
                into(),
                default(off),
                rc(),
                attributes(third_party(1, 2, 3)),
                attributes_fn(deny(unused)),
                attributes_impl(deny(unused)),
                fallible(off, error(crate::error::MyError)),
                lazy(off),
                get(name("get_")),
                get_mut(),
                set(),
                reader(off),
                writer(off),
                clearer(off),
                predicate(off),
                optional(),
                vis(pub(crate)),
                private(off),
                clone(off),
                copy(off),
                lock(off),
                inner_mut(off)
            )
        };

        assert_eq!(args.to_token_stream().to_string(), expected.to_string());
    }
}
