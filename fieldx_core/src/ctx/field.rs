mod fctx_props;

use delegate::delegate;
use fctx_props::FieldCTXProps;
use fieldx_aux::FXAccessorMode;
#[cfg(feature = "serde")]
use fieldx_aux::FXAttributes;
#[cfg(feature = "serde")]
use fieldx_aux::FXDefault;
use fieldx_aux::FXProp;
use once_cell::unsync::OnceCell;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use std::cell::Ref;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use syn::spanned::Spanned;

use crate::codegen::constructor::FXConstructor;
use crate::codegen::constructor::FXFieldConstructor;
use crate::field_receiver::props::FXFieldProps;
use crate::field_receiver::FXField;
use crate::types::helper::FXHelperKind;
use crate::types::impl_details::impl_async::FXAsyncImplementor;
use crate::types::impl_details::impl_plain::FXPlainImplementor;
use crate::types::impl_details::impl_sync::FXSyncImplementor;
use crate::types::impl_details::FXImplDetails;
use crate::types::meta::FXToksMeta;
use crate::types::FXInlining;

use super::codegen::FXImplementationContext;
use super::FXCodeGenCtx;

#[derive(Debug)]
pub struct FXFieldCtx<ImplCtx = ()>
where
    ImplCtx: FXImplementationContext,
{
    field:               FXField,
    #[allow(unused)]
    codegen_ctx:         Rc<FXCodeGenCtx<ImplCtx>>,
    constructor:         RefCell<Option<FXFieldConstructor>>,
    ty_wrapped:          OnceCell<TokenStream>,
    ident:               OnceCell<syn::Ident>,
    impl_details:        Box<dyn FXImplDetails<ImplCtx>>,
    #[cfg(feature = "serde")]
    default_fn_ident:    OnceCell<darling::Result<syn::Ident>>,
    builder_checker:     RefCell<Option<TokenStream>>,
    props:               fctx_props::FieldCTXProps<ImplCtx>,
    default_expr:        RefCell<Option<FXToksMeta>>,
    #[cfg(feature = "serde")]
    shadow_default_expr: RefCell<Option<FXToksMeta>>,
}

impl<ImplCtx> FXFieldCtx<ImplCtx>
where
    ImplCtx: FXImplementationContext,
{
    delegate! {
        to self.field {
            pub fn has_default_value(&self) -> bool;
            pub fn span(&self) -> Span;
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
            pub fn builder_method_optional(&self) -> FXProp<bool>;
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

    pub fn new(field: FXField, codegen_ctx: Rc<FXCodeGenCtx<ImplCtx>>) -> Self {
        let mut constructor = FXFieldConstructor::new(
            field.ident().expect("No field ident found").clone(),
            field.ty(),
            field.span(),
        );
        constructor.add_attributes(field.attrs().iter());
        let props = FieldCTXProps::<ImplCtx>::new(FXFieldProps::new(field.clone()), codegen_ctx.clone());

        let impl_details: Box<dyn FXImplDetails<ImplCtx>> = if *props.mode_async() {
            Box::new(FXAsyncImplementor)
        }
        else if *props.mode_sync() {
            Box::new(FXSyncImplementor)
        }
        else {
            Box::new(FXPlainImplementor)
        };

        // eprintln!(
        //     "FIELD ATTRS:\n{}",
        //     field
        //         .attrs()
        //         .iter()
        //         .map(|a| a.to_token_stream().to_string())
        //         .collect::<Vec<_>>()
        //         .join("\n")
        // );

        Self {
            props,
            constructor: RefCell::new(Some(constructor)),
            field,
            codegen_ctx,
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            impl_details,
            #[cfg(feature = "serde")]
            default_fn_ident: OnceCell::new(),
            builder_checker: RefCell::new(None),
            default_expr: RefCell::new(None),
            #[cfg(feature = "serde")]
            shadow_default_expr: RefCell::new(None),
        }
    }

    pub fn props(&self) -> &fctx_props::FieldCTXProps<ImplCtx> {
        &self.props
    }

    #[inline]
    #[allow(unused)]
    pub fn field(&self) -> &FXField {
        &self.field
    }

    #[inline]
    pub fn codegen_ctx(&self) -> &FXCodeGenCtx<ImplCtx> {
        &self.codegen_ctx
    }

    #[inline]
    pub fn take_constructor(&self) -> darling::Result<FXFieldConstructor> {
        self.constructor.borrow_mut().take().ok_or(darling::Error::custom(
            "Field constructor can't be given away because it's been done already; a post-code generation request was made"
        ))
    }

    #[inline(always)]
    pub fn skipped(&self) -> FXProp<bool> {
        self.props().field_props().skipped()
    }

    #[inline]
    pub fn ty_wrapped<F>(&self, initializer: F) -> darling::Result<&TokenStream>
    where
        F: FnOnce() -> darling::Result<TokenStream>,
    {
        self.ty_wrapped.get_or_try_init(initializer)
    }

    #[inline]
    pub fn ident(&self) -> &syn::Ident {
        self.ident
            .get_or_init(|| self.field.ident().expect("No field ident found"))
    }

    #[inline(always)]
    pub fn extra(&self) -> bool {
        self.field.is_extra()
    }

    #[cfg(feature = "serde")]
    pub fn default_fn_ident(&self) -> darling::Result<&syn::Ident> {
        self.default_fn_ident
            .get_or_init(|| {
                let field_ident = self.ident();
                let struct_ident = self.codegen_ctx.input_ident();
                Ok(self
                    .codegen_ctx
                    .unique_ident_pfx(&format!("__{struct_ident}_{field_ident}_default")))
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

    pub fn fallible_return_type<TT>(&self, fctx: &FXFieldCtx<ImplCtx>, ty: TT) -> darling::Result<TokenStream>
    where
        TT: ToTokens,
    {
        let ty = ty.to_token_stream();
        let fallible = fctx.fallible();
        Ok(if *fallible {
            let error_type = fctx.fallible_error();
            quote_spanned! {fallible.final_span()=> ::std::result::Result<#ty, #error_type>}
        }
        else {
            ty.to_token_stream()
        })
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
        let attrs = self
            .props
            .helper_attributes_fn(helper_kind)
            .map_or(Vec::new(), |a| a.iter().map(|a| &**a).collect::<Vec<&syn::Attribute>>());

        match inlining {
            FXInlining::Default => quote_spanned![span=> #( #attrs )*],
            FXInlining::Inline => quote_spanned![span=> #[inline] #( #attrs )*],
            FXInlining::Always => quote_spanned![span=> #[inline(always)] #( #attrs )*],
        }
    }

    pub fn set_default_expr<FT: Into<FXToksMeta>>(&self, expr: FT) {
        *self.default_expr.borrow_mut() = Some(expr.into());
    }

    pub fn default_expr(&self) -> Ref<Option<FXToksMeta>> {
        self.default_expr.borrow()
    }

    #[cfg(feature = "serde")]
    pub fn set_shadow_default_expr<FT: Into<FXToksMeta>>(&self, expr: FT) {
        *self.shadow_default_expr.borrow_mut() = Some(expr.into());
    }

    #[cfg(feature = "serde")]
    pub fn shadow_default_expr(&self) -> Ref<Option<FXToksMeta>> {
        self.shadow_default_expr.borrow()
    }

    pub fn impl_details(&self) -> &dyn FXImplDetails<ImplCtx> {
        &self.impl_details
    }
}
