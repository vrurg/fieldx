use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::rc::Rc;
use std::rc::Weak;

use fieldx_core::codegen::constructor::FXConstructor;
use fieldx_core::codegen::constructor::FXFieldConstructor;
use fieldx_core::codegen::constructor::FXFnConstructor;
use fieldx_core::codegen::constructor::FXStructConstructor;
use fieldx_core::ctx::codegen::FXImplementationContext;
use fieldx_core::ctx::FXCodeGenCtx;
use fieldx_core::ctx::FXFieldCtx;
use once_cell::unsync::OnceCell;
use quote::quote_spanned;

pub(crate) type FXDeriveCodegenCtx = FXCodeGenCtx<FXDeriveMacroCtx>;
pub(crate) type FXDeriveFieldCtx = FXFieldCtx<FXDeriveMacroCtx>;

#[derive(Debug)]
pub(crate) struct FXDeriveMacroCtx {
    codegen_ctx: Weak<FXCodeGenCtx<Self>>,

    builder_struct: OnceCell<RefCell<FXStructConstructor>>,

    #[cfg(feature = "serde")]
    shadow_struct: RefCell<Option<FXStructConstructor>>,

    #[cfg(feature = "serde")]
    shadow_var_ident: OnceCell<syn::Ident>,
    #[cfg(feature = "serde")]
    me_var_ident:     OnceCell<syn::Ident>,

    copyable_types: RefCell<Vec<syn::Type>>,
}

impl FXDeriveMacroCtx {
    pub(crate) fn new() -> Self {
        Self {
            codegen_ctx:                                Weak::new(),
            builder_struct:                             OnceCell::new(),
            #[cfg(feature = "serde")]
            shadow_struct:                              RefCell::new(None),
            copyable_types:                             RefCell::new(vec![]),
            #[cfg(feature = "serde")]
            shadow_var_ident:                           OnceCell::new(),
            #[cfg(feature = "serde")]
            me_var_ident:                               OnceCell::new(),
        }
    }

    pub(crate) fn codegen_ctx(&self) -> darling::Result<Rc<FXCodeGenCtx<Self>>> {
        self.codegen_ctx
            .upgrade()
            .ok_or_else(|| darling::Error::custom("Codegen context is gone or not set yet"))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn set_shadow_struct(&self, shadow_struct: FXStructConstructor) {
        self.shadow_struct.replace(Some(shadow_struct));
    }

    #[cfg(feature = "serde")]
    pub(crate) fn shadow_struct<'a>(&'a self) -> darling::Result<Ref<'a, FXStructConstructor>> {
        let sstruct = self.shadow_struct.borrow();
        if sstruct.is_none() {
            return Err(darling::Error::custom("Shadow struct is not set yet"));
        }
        Ok(Ref::map(sstruct, |s| s.as_ref().unwrap()))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn shadow_struct_mut<'a>(&'a self) -> darling::Result<RefMut<'a, FXStructConstructor>> {
        let sstruct = self.shadow_struct.borrow_mut();
        if sstruct.is_none() {
            return Err(darling::Error::custom("Shadow struct is not set yet"));
        }
        Ok(RefMut::map(sstruct, |s| s.as_mut().unwrap()))
    }

    #[inline(always)]
    pub(crate) fn copyable_types<'a>(&'a self) -> std::cell::Ref<'a, Vec<syn::Type>> {
        self.copyable_types.borrow()
    }

    #[inline(always)]
    pub(crate) fn add_for_copy_trait_check(&self, field_ctx: &FXFieldCtx<Self>) {
        self.copyable_types.borrow_mut().push(field_ctx.ty().clone());
    }

    #[cfg(feature = "serde")]
    #[inline]
    // How to reference shadow instance in an associated function
    pub(crate) fn shadow_var_ident(&self) -> darling::Result<&syn::Ident> {
        use quote::format_ident;

        self.shadow_var_ident.get_or_try_init(|| {
            let ctx = self.codegen_ctx()?;
            Ok(format_ident!("__shadow", span = ctx.arg_props().serde().final_span()))
        })
    }

    // How to reference struct instance in an associated function
    #[cfg(feature = "serde")]
    #[inline]
    pub(crate) fn me_var_ident(&self) -> darling::Result<&syn::Ident> {
        use quote::format_ident;

        self.me_var_ident.get_or_try_init(|| {
            let ctx = self.codegen_ctx()?;
            Ok(format_ident!("__me", span = ctx.arg_props().serde().final_span()))
        })
    }

    #[inline(always)]
    pub(crate) fn add_builder_method(&self, builder: FXFnConstructor) -> darling::Result<&Self> {
        self.builder_struct_mut()?.struct_impl_mut().add_method(builder);
        Ok(self)
    }

    #[inline(always)]
    pub(crate) fn add_builder_field(&self, builder_field: FXFieldConstructor) -> darling::Result<&Self> {
        self.builder_struct_mut()?.add_field(builder_field);
        Ok(self)
    }

    pub(crate) fn builder_struct<'a>(&'a self) -> darling::Result<Ref<'a, FXStructConstructor>> {
        Ok(self._builder_struct()?.borrow())
    }

    pub(crate) fn builder_struct_mut<'a>(&'a self) -> darling::Result<RefMut<'a, FXStructConstructor>> {
        Ok(self._builder_struct()?.borrow_mut())
    }

    fn _builder_struct(&self) -> darling::Result<&RefCell<FXStructConstructor>> {
        let ctx = self.codegen_ctx()?;
        let arg_props = ctx.arg_props();
        let prop = arg_props.builder_struct();
        if *prop {
            Ok(self
                .builder_struct
                .get_or_try_init(|| -> darling::Result<RefCell<FXStructConstructor>> {
                    let builder_struct = RefCell::new(FXStructConstructor::new(arg_props.builder_ident().clone()));
                    {
                        let mut bs_mut = builder_struct.borrow_mut();
                        bs_mut
                            .set_vis(arg_props.builder_struct_visibility())
                            .set_generics(ctx.input().generics().clone())
                            .set_span(prop.final_span())
                            .maybe_add_attributes(arg_props.builder_struct_attributes().map(|a| a.iter()))
                            .struct_impl_mut()
                            .maybe_add_attributes(arg_props.builder_struct_attributes_impl().map(|a| a.iter()));

                        if *arg_props.builder_default() {
                            bs_mut.add_attribute_toks(quote_spanned! {prop.final_span()=> #[derive(Default)]})?;
                        }
                    }

                    Ok(builder_struct)
                })?)
        }
        else {
            Err(darling::Error::custom("Builder struct is not enabled"))
        }
    }
}

impl FXImplementationContext for FXDeriveMacroCtx {
    fn set_codegen_ctx(&mut self, ctx: Weak<FXCodeGenCtx<Self>>) {
        self.codegen_ctx = ctx;
    }
}
