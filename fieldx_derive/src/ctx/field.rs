mod derived_props;

use super::FXCodeGenCtx;
use crate::{
    codegen::{constructor::field::FieldConstructor, FXInlining},
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
    cell::{OnceCell, RefCell, RefMut},
    rc::Rc,
};

#[derive(Debug)]
pub struct FXFieldCtx {
    field:            FXField,
    #[allow(unused)]
    codegen_ctx:      Rc<FXCodeGenCtx>,
    constructor:      RefCell<FieldConstructor>,
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
            pub fn has_default_value(&self) -> bool;
            pub fn span(&self) -> &Span;
            pub fn ty(&self) -> &syn::Type;
            pub fn vis(&self) -> &syn::Visibility;
        }
    }

    delegate! {
        to self.props {
            pub fn accessor(&self) -> FXProp<bool>;
            pub fn accessor_ident(&self) -> &syn::Ident;
            pub fn accessor_mode(&self) -> &FXProp<FXAccessorMode>;
            pub fn accessor_mut(&self) -> FXProp<bool>;
            pub fn accessor_mut_ident(&self) -> &syn::Ident;
            pub fn accessor_mut_visibility(&self) -> &syn::Visibility;
            pub fn accessor_visibility(&self) -> &syn::Visibility;
            pub fn builder_ident(&self) -> &syn::Ident;
            pub fn builder(&self) -> FXProp<bool>;
            pub fn builder_required(&self) -> FXProp<bool>;
            pub fn builder_into(&self) -> FXProp<bool>;
            pub fn builder_method_visibility(&self) -> &syn::Visibility;
            pub fn clearer(&self) -> FXProp<bool>;
            pub fn clearer_ident(&self) -> &syn::Ident;
            pub fn clearer_visibility(&self) -> &syn::Visibility;
            pub fn default_value(&self) -> Option<&syn::Expr>;
            pub fn fallible(&self) -> FXProp<bool>;
            pub fn fallible_error(&self) -> Option<&syn::Path>;
            pub fn forced_builder(&self) -> FXProp<bool>;
            pub fn inner_mut(&self) -> FXProp<bool>;
            pub fn lazy(&self) -> FXProp<bool>;
            pub fn lazy_ident(&self) -> &syn::Ident;
            pub fn lock(&self) -> FXProp<bool>;
            pub fn mode_async(&self) -> FXProp<bool>;
            pub fn mode_plain(&self) -> FXProp<bool>;
            pub fn mode_sync(&self) -> FXProp<bool>;
            pub fn optional(&self) -> FXProp<bool>;
            pub fn predicate(&self) -> FXProp<bool>;
            pub fn predicate_ident(&self) -> &syn::Ident;
            pub fn predicate_visibility(&self) -> &syn::Visibility;
            pub fn reader(&self) -> FXProp<bool>;
            pub fn reader_ident(&self) -> &syn::Ident;
            pub fn reader_visibility(&self) -> &syn::Visibility;
            pub fn setter(&self) -> FXProp<bool>;
            pub fn setter_ident(&self) -> &syn::Ident;
            pub fn setter_into(&self) -> FXProp<bool>;
            pub fn setter_visibility(&self) -> &syn::Visibility;
            pub fn writer(&self) -> FXProp<bool>;
            pub fn writer_ident(&self) -> &syn::Ident;
            pub fn writer_visibility(&self) -> &syn::Visibility;

            #[cfg(feature = "serde")]
            pub fn serde(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub fn serialize(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub fn deserialize(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub fn serde_optional(&self) -> FXProp<bool>;
            #[cfg(feature = "serde")]
            pub fn serde_default_value(&self) -> Option<&FXDefault>;
            #[cfg(feature = "serde")]
            pub fn serde_attributes(&self) -> Option<&FXAttributes>;
            #[cfg(feature = "serde")]
            pub fn serde_rename_serialize(&self) -> Option<&FXProp<String>>;
            #[cfg(feature = "serde")]
            pub fn serde_rename_deserialize(&self) -> Option<&FXProp<String>>;
        }
    }

    // arg_accessor! { optional: FXBool, lock: FXBool, inner_mut: FXBool }

    pub fn new(field: FXField, codegen_ctx: Rc<FXCodeGenCtx>) -> Self {
        let mut constructor = FieldConstructor::new(
            field.ident().expect("No field ident found").clone(),
            field.ty(),
            *field.span(),
        );
        constructor.add_attributes(field.attrs().iter());

        Self {
            props: FieldCTXProps::new(FXFieldProps::new(field.clone()), codegen_ctx.clone()),
            constructor: RefCell::new(constructor),
            field,
            codegen_ctx,
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            #[cfg(feature = "serde")]
            default_fn_ident: OnceCell::new(),
            builder_checker: RefCell::new(None),
        }
    }

    pub fn props(&self) -> &derived_props::FieldCTXProps {
        &self.props
    }

    #[inline]
    #[allow(unused)]
    pub fn field(&self) -> &FXField {
        &self.field
    }

    #[inline]
    pub fn constructor(&self) -> RefMut<FieldConstructor> {
        self.constructor.borrow_mut()
    }

    #[inline(always)]
    pub fn skipped(&self) -> FXProp<bool> {
        self.props().field_props().skipped()
    }

    #[inline]
    pub fn ty_wrapped<F>(&self, initializer: F) -> &TokenStream
    where
        F: FnOnce() -> TokenStream,
    {
        self.ty_wrapped.get_or_init(initializer)
    }

    #[inline]
    pub fn ident(&self) -> &syn::Ident {
        self.ident
            .get_or_init(|| self.field.ident().expect("No field ident found"))
    }

    #[cfg(feature = "serde")]
    pub fn default_fn_ident<'s>(&'s self) -> darling::Result<&'s syn::Ident> {
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

    pub fn set_builder_checker(&self, bc: TokenStream) {
        *self.builder_checker.borrow_mut() = Some(bc);
    }

    pub fn builder_checker(&self) -> Option<TokenStream> {
        self.builder_checker.borrow().clone()
    }

    pub fn fallible_shortcut(&self) -> TokenStream {
        let fallible = self.props.fallible();
        if *fallible {
            quote_spanned! {fallible.final_span()=> ?}
        }
        else {
            quote![]
        }
    }

    pub fn fallible_ok_return<T: ToTokens>(&self, ret: &T) -> TokenStream {
        let fallible = self.props.fallible();
        if *fallible {
            quote_spanned! {fallible.final_span()=> Ok(#ret)}
        }
        else {
            ret.to_token_stream()
        }
    }

    pub fn helper_attributes_fn(&self, helper_kind: FXHelperKind, inlining: FXInlining, span: Span) -> TokenStream {
        let attrs = self.props.helper_attributes_fn(helper_kind);

        match inlining {
            FXInlining::Default => attrs.map_or(quote![], |a| quote![#a]),
            FXInlining::Inline => quote_spanned![span=> #[inline] #attrs],
            FXInlining::Always => quote_spanned![span=> #[inline(always)] #attrs],
        }
    }
}
