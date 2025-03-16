mod derived_props;

use super::FXCodeGenCtx;
use crate::{
    codegen::{
        constructor::{field::FXFieldConstructor, FXConstructor},
        FXInlining,
    },
    fields::{FXField, FXFieldProps},
    helper::FXHelperKind,
};
use delegate::delegate;
use derived_props::FieldCTXProps;
use fieldx_aux::{FXAccessorMode, FXProp};
#[cfg(feature = "serde")]
use fieldx_aux::{FXAttributes, FXDefault};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};

#[derive(Debug)]
pub(crate) struct FXFieldCtx {
    field:            FXField,
    #[allow(unused)]
    codegen_ctx:      Rc<FXCodeGenCtx>,
    constructor:      RefCell<Option<FXFieldConstructor>>,
    ty_wrapped:       OnceCell<TokenStream>,
    ident:            OnceCell<syn::Ident>,
    #[cfg(feature = "serde")]
    default_fn_ident: OnceCell<darling::Result<syn::Ident>>,
    builder_checker:  RefCell<Option<TokenStream>>,
    props:            derived_props::FieldCTXProps,
}

impl FXFieldCtx {
    delegate! {
        to self.field {
            pub(crate) fn has_default_value(&self) -> bool;
            pub(crate) fn span(&self) -> &Span;
            pub(crate) fn ty(&self) -> &syn::Type;
            pub(crate) fn vis(&self) -> &syn::Visibility;
        }
    }

    delegate! {
        to self.props {
            pub(crate) fn accessor(&self) -> FXProp<bool>;
            pub(crate) fn accessor_ident(&self) -> &syn::Ident;
            pub(crate) fn accessor_mode(&self) -> &FXProp<FXAccessorMode>;
            pub(crate) fn accessor_mut(&self) -> FXProp<bool>;
            pub(crate) fn accessor_mut_ident(&self) -> &syn::Ident;
            pub(crate) fn accessor_mut_visibility(&self) -> &syn::Visibility;
            pub(crate) fn accessor_visibility(&self) -> &syn::Visibility;
            pub(crate) fn builder_ident(&self) -> &syn::Ident;
            pub(crate) fn builder(&self) -> FXProp<bool>;
            pub(crate) fn builder_required(&self) -> FXProp<bool>;
            pub(crate) fn builder_into(&self) -> FXProp<bool>;
            pub(crate) fn builder_method_visibility(&self) -> &syn::Visibility;
            pub(crate) fn clearer(&self) -> FXProp<bool>;
            pub(crate) fn clearer_ident(&self) -> &syn::Ident;
            pub(crate) fn clearer_visibility(&self) -> &syn::Visibility;
            pub(crate) fn default_value(&self) -> Option<&syn::Expr>;
            pub(crate) fn fallible(&self) -> FXProp<bool>;
            pub(crate) fn fallible_error(&self) -> Option<&syn::Path>;
            pub(crate) fn forced_builder(&self) -> FXProp<bool>;
            pub(crate) fn inner_mut(&self) -> FXProp<bool>;
            pub(crate) fn lazy(&self) -> FXProp<bool>;
            pub(crate) fn lazy_ident(&self) -> &syn::Ident;
            pub(crate) fn lock(&self) -> FXProp<bool>;
            pub(crate) fn mode_async(&self) -> FXProp<bool>;
            pub(crate) fn mode_plain(&self) -> FXProp<bool>;
            pub(crate) fn mode_sync(&self) -> FXProp<bool>;
            pub(crate) fn optional(&self) -> FXProp<bool>;
            pub(crate) fn predicate(&self) -> FXProp<bool>;
            pub(crate) fn predicate_ident(&self) -> &syn::Ident;
            pub(crate) fn predicate_visibility(&self) -> &syn::Visibility;
            pub(crate) fn reader(&self) -> FXProp<bool>;
            pub(crate) fn reader_ident(&self) -> &syn::Ident;
            pub(crate) fn reader_visibility(&self) -> &syn::Visibility;
            pub(crate) fn setter(&self) -> FXProp<bool>;
            pub(crate) fn setter_ident(&self) -> &syn::Ident;
            pub(crate) fn setter_into(&self) -> FXProp<bool>;
            pub(crate) fn setter_visibility(&self) -> &syn::Visibility;
            pub(crate) fn writer(&self) -> FXProp<bool>;
            pub(crate) fn writer_ident(&self) -> &syn::Ident;
            pub(crate) fn writer_visibility(&self) -> &syn::Visibility;

            #[cfg(feature = "serde")]
            pub(crate) fn serde(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub(crate) fn serialize(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub(crate) fn deserialize(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub(crate) fn serde_optional(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub(crate) fn serde_default_value(&self) -> Option<&FXDefault>;
            #[cfg(feature = "serde")]
            pub(crate) fn serde_attributes(&self) -> Option<&FXAttributes>;
            #[cfg(feature = "serde")]
            pub(crate) fn serde_rename_serialize(&self) -> Option<&FXProp<String>>;
            #[cfg(feature = "serde")]
            pub(crate) fn serde_rename_deserialize(&self) -> Option<&FXProp<String>>;
        }
    }

    // arg_accessor! { optional: FXBool, lock: FXBool, inner_mut: FXBool }

    pub(crate) fn new(field: FXField, codegen_ctx: Rc<FXCodeGenCtx>) -> Self {
        let mut constructor = FXFieldConstructor::new(
            field.ident().expect("No field ident found").clone(),
            field.ty(),
            *field.span(),
        );
        constructor.add_attributes(field.attrs().iter());

        Self {
            props: FieldCTXProps::new(FXFieldProps::new(field.clone()), codegen_ctx.clone()),
            constructor: RefCell::new(Some(constructor)),
            field,
            codegen_ctx,
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            #[cfg(feature = "serde")]
            default_fn_ident: OnceCell::new(),
            builder_checker: RefCell::new(None),
        }
    }

    pub(crate) fn props(&self) -> &derived_props::FieldCTXProps {
        &self.props
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn field(&self) -> &FXField {
        &self.field
    }

    #[inline]
    pub(crate) fn take_constructor(&self) -> darling::Result<FXFieldConstructor> {
        self.constructor.borrow_mut().take().ok_or(darling::Error::custom(
            "Field constructor can't be given away because it's been done already; a post-code generation request was made"
        ))
    }

    #[inline(always)]
    pub(crate) fn skipped(&self) -> FXProp<bool> {
        self.props().field_props().skipped()
    }

    #[inline]
    pub(crate) fn ty_wrapped<F>(&self, initializer: F) -> &TokenStream
    where
        F: FnOnce() -> TokenStream,
    {
        self.ty_wrapped.get_or_init(initializer)
    }

    #[inline]
    pub(crate) fn ident(&self) -> &syn::Ident {
        self.ident
            .get_or_init(|| self.field.ident().expect("No field ident found"))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn default_fn_ident<'s>(&'s self) -> darling::Result<&'s syn::Ident> {
        self.default_fn_ident
            .get_or_init(|| {
                let field_ident = self.ident();
                let struct_ident = self.codegen_ctx.input_ident();
                Ok(self
                    .codegen_ctx
                    .unique_ident_pfx(&format!("__{}_{}_default", struct_ident, field_ident)))
            })
            .as_ref()
            .map_err(
                // Normally, the error is cloned only once since the upstream will give up and not attempt to request
                // the identifier again.
                |e| e.clone(),
            )
    }

    pub(crate) fn set_builder_checker(&self, bc: TokenStream) {
        *self.builder_checker.borrow_mut() = Some(bc);
    }

    pub(crate) fn builder_checker(&self) -> Option<TokenStream> {
        self.builder_checker.borrow().clone()
    }

    pub(crate) fn fallible_shortcut(&self) -> TokenStream {
        let fallible = self.props.fallible();
        if *fallible {
            quote_spanned! {fallible.final_span()=> ?}
        }
        else {
            quote![]
        }
    }

    pub(crate) fn fallible_ok_return<T: ToTokens>(&self, ret: &T) -> TokenStream {
        let fallible = self.props.fallible();
        if *fallible {
            quote_spanned! {fallible.final_span()=> Ok(#ret)}
        }
        else {
            ret.to_token_stream()
        }
    }

    pub(crate) fn helper_attributes_fn(
        &self,
        helper_kind: FXHelperKind,
        inlining: FXInlining,
        span: Span,
    ) -> TokenStream {
        let attrs = self.props.helper_attributes_fn(helper_kind);

        match inlining {
            FXInlining::Default => attrs.map_or(quote![], |a| quote![#a]),
            FXInlining::Inline => quote_spanned![span=> #[inline] #attrs],
            FXInlining::Always => quote_spanned![span=> #[inline(always)] #attrs],
        }
    }
}
