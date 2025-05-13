use super::FXFieldCtx;
use crate::codegen::constructor::FXConstructor;
use crate::codegen::constructor::FXFieldConstructor;
use crate::codegen::constructor::FXFnConstructor;
use crate::codegen::constructor::FXStructConstructor;
use crate::field_receiver::FXField;
use crate::struct_receiver::args::props::FXStructArgProps;
use crate::struct_receiver::args::FXStructArgs;
use crate::struct_receiver::FXStructReceiver;
use crate::types::impl_details::impl_async::FXAsyncImplementor;
use crate::types::impl_details::impl_plain::FXPlainImplementor;
use crate::types::impl_details::impl_sync::FXSyncImplementor;
use crate::types::impl_details::FXImplDetails;
use delegate::delegate;
use fieldx_aux::FXProp;
use getset::CopyGetters;
use getset::Getters;
use once_cell::unsync::OnceCell;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::rc::Weak;

pub trait FXImplementationContext: Debug + Sized {
    fn set_codegen_ctx(&mut self, ctx: Weak<FXCodeGenCtx<Self>>);
}

impl FXImplementationContext for () {
    fn set_codegen_ctx(&mut self, _ctx: Weak<FXCodeGenCtx<Self>>) {}
}

#[derive(Debug, Getters, CopyGetters)]
// ImplCtx specify implementation-specific extra context type.
pub struct FXCodeGenCtx<ImplCtx = ()>
where
    ImplCtx: FXImplementationContext,
{
    myself: Weak<Self>,

    errors: OnceCell<RefCell<darling::error::Accumulator>>,
    tokens: OnceCell<RefCell<TokenStream>>,

    user_struct: RefCell<FXStructConstructor>,

    #[getset(get = "pub")]
    args: FXStructArgs,

    #[getset(skip)]
    arg_props:    Rc<FXStructArgProps<ImplCtx>>,
    // Use Option because FXInputReceiver can't implement Default
    input:        Option<Rc<FXStructReceiver>>,
    extra_fields: RefCell<Vec<FXField>>,
    unique_id:    RefCell<u32>,
    // Unlike the field-level context, struct properties depend on the struct-level context itself.
    // Therefore, property-based initializations are performed lazily for enhanced safety.
    impl_details: OnceCell<Box<dyn FXImplDetails<ImplCtx>>>,

    field_ctx_table: OnceCell<RefCell<HashMap<syn::Ident, Rc<FXFieldCtx<ImplCtx>>>>>,

    impl_ctx: ImplCtx,
}

impl<ImplCtx> FXCodeGenCtx<ImplCtx>
where
    ImplCtx: FXImplementationContext,
{
    delegate! {
        /// Delegate to the FXArgProps implementation.
        to self.arg_props {
            pub fn needs_default(&self) -> FXProp<bool>;
            pub fn syncish(&self) -> FXProp<bool>;
            pub fn needs_new(&self) -> FXProp<bool>;
        }
    }

    pub fn new(input: FXStructReceiver, args: FXStructArgs, mut impl_ctx: ImplCtx) -> Rc<Self> {
        let input = Rc::new(input);

        let mut user_struct = FXStructConstructor::new(input.ident().clone());
        user_struct
            .set_span(input.ident().span())
            .set_generics(input.generics().clone())
            .set_vis(input.vis().clone())
            .add_attributes(input.attrs().iter());

        Rc::new_cyclic(|myself| {
            impl_ctx.set_codegen_ctx(myself.clone());
            Self {
                myself: myself.clone(),
                arg_props: Rc::new(FXStructArgProps::<ImplCtx>::new(args.clone(), myself.clone())),
                args,

                user_struct: RefCell::new(user_struct),

                errors: OnceCell::new(),
                extra_fields: RefCell::new(Vec::new()),
                field_ctx_table: OnceCell::new(),
                tokens: OnceCell::new(),
                unique_id: RefCell::new(0),
                impl_details: OnceCell::new(),

                input: Some(input),
                impl_ctx,
            }
        })
    }

    pub fn user_struct(&self) -> Ref<FXStructConstructor> {
        self.user_struct.borrow()
    }

    pub fn user_struct_mut(&self) -> RefMut<FXStructConstructor> {
        self.user_struct.borrow_mut()
    }

    fn myself(&self) -> Rc<Self> {
        self.myself
            .upgrade()
            .expect("Context object is gone while trying to upgrade a weak reference")
    }

    pub fn impl_ctx(&self) -> &ImplCtx {
        &self.impl_ctx
    }

    #[inline(always)]
    pub fn input(&self) -> &FXStructReceiver {
        // Unwrap is safe because the field is always set by the constructor and can't be taken away.
        self.input.as_ref().unwrap()
    }

    pub fn arg_props(&self) -> &Rc<FXStructArgProps<ImplCtx>> {
        &self.arg_props
    }

    // #[inline(always)]
    // #[cfg(feature = "serde")]
    // pub fn shadow_fields(&self) -> std::cell::Ref<Vec<TokenStream>> {
    //     self.shadow_field_toks.borrow()
    // }

    // #[inline(always)]
    // #[cfg(feature = "serde")]
    // pub fn shadow_defaults(&self) -> std::cell::Ref<Vec<TokenStream>> {
    //     self.shadow_default_toks.borrow()
    // }

    #[inline(always)]
    pub fn push_error(&self, err: darling::Error) {
        self._errors().borrow_mut().push(err);
    }

    #[inline(always)]
    pub fn ok_or_empty<T: From<TokenStream>>(&self, outcome: darling::Result<T>) -> T {
        outcome.unwrap_or_else(|err| {
            self.push_error(err);
            quote![].into()
        })
    }

    #[inline(always)]
    pub fn ok_or_record<T>(&self, outcome: darling::Result<T>) {
        if let Err(err) = outcome {
            self.push_error(err);
        }
    }

    pub fn exec_or_record(&self, code: impl FnOnce() -> darling::Result<()>) {
        if let Err(err) = code() {
            self.push_error(err)
        }
    }

    #[inline(always)]
    pub fn add_field_decl(&self, field: FXFieldConstructor) {
        self.user_struct.borrow_mut().add_field(field);
    }

    #[inline(always)]
    pub fn add_method(&self, method: FXFnConstructor) -> &Self {
        self.user_struct.borrow_mut().struct_impl_mut().add_method(method);
        self
    }

    #[inline(always)]
    pub fn maybe_add_method(&self, method: Option<FXFnConstructor>) -> &Self {
        if let Some(method) = method {
            self.user_struct.borrow_mut().struct_impl_mut().add_method(method);
        }
        self
    }

    #[inline(always)]
    pub fn input_ident(&self) -> &syn::Ident {
        self.input().ident()
    }

    #[inline(always)]
    pub fn struct_type_toks(&self) -> TokenStream {
        let struct_input = self.input();
        let ident = struct_input.ident();
        let (_, generic_params, _) = struct_input.generics().split_for_impl();
        quote_spanned! {ident.span()=>
            #ident #generic_params
        }
    }

    pub fn tokens_extend<T: ToTokens>(&self, toks: T) {
        self._tokens().borrow_mut().extend(toks.to_token_stream());
    }

    #[inline]
    pub fn finalize(&self) -> TokenStream {
        if let Some(errors) = self.errors.get().map(|e| e.take()) {
            match errors.finish() {
                Ok(_) => (),
                Err(err) => {
                    self.tokens_extend(err.write_errors());
                }
            };
        }

        if let Some(tokens) = self.tokens.get() {
            tokens.take()
        }
        else {
            quote![]
        }
    }

    #[inline]
    pub fn add_extra_field(&self, field: FXField) {
        self.extra_fields.borrow_mut().push(field);
    }

    pub fn field_ctx_table(&self) -> &RefCell<HashMap<syn::Ident, Rc<FXFieldCtx<ImplCtx>>>> {
        self.field_ctx_table.get_or_init(|| {
            RefCell::new(
                self.extra_fields
                    .borrow()
                    .iter()
                    .chain(self.input().fields().iter().cloned())
                    .map(|f| {
                        let field_ident = f.ident().unwrap();
                        (
                            field_ident.clone(),
                            Rc::new(FXFieldCtx::<ImplCtx>::new(f.clone(), self.myself())),
                        )
                    })
                    .collect(),
            )
        })
    }

    pub fn ident_field_ctx(self: &Rc<Self>, field_ident: &syn::Ident) -> darling::Result<Rc<FXFieldCtx<ImplCtx>>> {
        Ok(self
            .field_ctx_table()
            .borrow()
            .get(field_ident)
            .ok_or(darling::Error::custom(format!(
                "Field '{field_ident}' not found in context table"
            )))?
            .clone())
    }

    pub fn field_ctx(&self, field: &FXField) -> Rc<FXFieldCtx<ImplCtx>> {
        let mut field_ctx_table = self.field_ctx_table().borrow_mut();
        field_ctx_table
            .entry(field.ident().unwrap().clone())
            .or_insert_with(|| Rc::new(FXFieldCtx::new(field.clone(), self.myself())))
            .clone()
    }

    pub fn all_field_ctx(&self) -> Vec<Rc<FXFieldCtx<ImplCtx>>> {
        // Don't iterate over field_ctx_table keys because we want to preserve the order of fields as they appear in the
        // struct.
        self.extra_fields
            .borrow()
            .iter()
            .chain(self.input().fields().iter().cloned())
            .map(|f| self.field_ctx(f))
            .collect()
    }

    #[inline(always)]
    pub fn struct_generic_params(&self) -> TokenStream {
        self.input().generics().split_for_impl().1.to_token_stream()
    }

    #[allow(dead_code)]
    #[inline]
    pub fn unique_ident_pfx(&self, prefix: &str) -> syn::Ident {
        let new_count = *self.unique_id.borrow() + 1;
        let _ = self.unique_id.replace(new_count);
        format_ident!("{}_{}", prefix, new_count)
    }

    #[allow(dead_code)]
    #[inline]
    pub fn unique_ident(&self) -> syn::Ident {
        self.unique_ident_pfx(&format!("__{}_fxsym", self.input_ident()))
    }

    /// The implementation details for the struct are determined by the following rules:
    /// 1. Use [`FXAsyncImplementor`] if the struct-level `async` mode is enabled explicitly.
    /// 2. Use [`FXSyncImplementor`] if the struct-level `sync` mode is enabled explicitly, or if any of the fields is
    ///    marked as either `sync` or `async`.
    /// 3. Otherwise, use [`FXPlainImplementor`].
    pub fn impl_details(&self) -> &dyn FXImplDetails<ImplCtx> {
        self.impl_details
            .get_or_init(|| {
                let arg_props = self.arg_props();

                if arg_props.mode_async().is_some_and(|p| *p) {
                    Box::new(FXAsyncImplementor)
                }
                else if arg_props.mode_sync().is_some_and(|p| *p) || *arg_props.syncish() {
                    Box::new(FXSyncImplementor)
                }
                else {
                    Box::new(FXPlainImplementor)
                }
            })
            .as_ref()
    }

    #[inline(always)]
    fn _errors(&self) -> &RefCell<darling::error::Accumulator> {
        self.errors.get_or_init(|| RefCell::new(darling::Error::accumulator()))
    }

    #[inline(always)]
    fn _tokens(&self) -> &RefCell<TokenStream> {
        self.tokens.get_or_init(|| RefCell::new(TokenStream::new()))
    }
}
