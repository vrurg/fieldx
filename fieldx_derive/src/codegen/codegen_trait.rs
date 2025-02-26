use super::{
    method_constructor::MethodConstructor, FXCodeGenCtx, FXCodeGenPlain, FXCodeGenSync, FXFieldCtx, FXInlining,
    FXValueRepr,
};
use crate::{helper::*, FXInputReceiver};
use enum_dispatch::enum_dispatch;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::{spanned::Spanned, Ident};

#[enum_dispatch]
pub enum FXCodeGenerator<'a> {
    ModePlain(FXCodeGenPlain<'a>),
    ModeSync(FXCodeGenSync<'a>),
}

#[enum_dispatch(FXCodeGenerator)]
pub trait FXCodeGenContextual {
    fn ctx(&self) -> &Rc<FXCodeGenCtx>;

    // Actual code producers
    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_lazy_builder_wrapper(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: FXValueRepr<TokenStream>) -> darling::Result<TokenStream>;
    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_lazy_initializer(
        &self,
        fctx: &FXFieldCtx,
        method_constructor: &mut MethodConstructor,
    ) -> darling::Result<TokenStream>;
    #[cfg(feature = "serde")]
    // How to move field from shadow struct
    fn field_from_shadow(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    #[cfg(feature = "serde")]
    // How to move field from the struct itself
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;

    // fn add_field_decl(&self, field: TokenStream);
    // fn add_defaults_decl(&self, defaults: TokenStream);
    // fn add_method_decl(&self, method: TokenStream);
    // fn add_builder_decl(&self, builder_method: TokenStream);
    // fn add_builder_field_decl(&self, builder_field: TokenStream);
    // fn add_builder_field_ident(&self, fctx: syn::Ident);
    // fn add_for_copy_trait_check(&self, fctx: &FXFieldCtx);
    // #[cfg(feature = "serde")]
    // fn add_shadow_field_decl(&self, field: TokenStream);
    // #[cfg(feature = "serde")]
    // fn add_shadow_default_decl(&self, field: TokenStream);

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> darling::Result<&'s TokenStream>;
    fn ref_count_types(&self) -> (TokenStream, TokenStream);
    // fn copyable_types(&self) -> Ref<Vec<syn::Type>>;
    // #[cfg(feature = "serde")]
    // fn shadow_fields(&self) -> Ref<Vec<TokenStream>>;
    // #[cfg(feature = "serde")]
    // fn shadow_defaults(&self) -> Ref<Vec<TokenStream>>;

    // fn field_ctx_table(&self) -> Ref<HashMap<Ident, FXFieldCtx>>;
    // fn field_ctx_table_mut(&self) -> RefMut<HashMap<Ident, FXFieldCtx>>;
    // fn builder_field_ident(&self) -> &RefCell<Vec<syn::Ident>>;
    // fn methods_combined(&self) -> TokenStream;
    // fn defaults_combined(&self) -> TokenStream;
    // fn builder_fields_combined(&self) -> TokenStream;
    // fn builders_combined(&self) -> TokenStream;
    // fn struct_fields(&self) -> Ref<Vec<TokenStream>>;

    // Common implementations
    fn input(&self) -> &FXInputReceiver {
        &self.ctx().input()
    }

    fn ok_or_empty(&self, outcome: darling::Result<TokenStream>) -> TokenStream {
        self.ok_or_else(outcome, || quote![])
    }

    fn ok_or_else<T>(&self, outcome: darling::Result<T>, mapper: impl FnOnce() -> T) -> T {
        outcome.unwrap_or_else(|err| {
            self.ctx().push_error(err);
            mapper()
        })
    }

    fn ok_or_record(&self, outcome: darling::Result<()>) {
        if let Err(err) = outcome {
            self.ctx().push_error(err)
        }
    }

    fn helper_span(&self, fctx: &FXFieldCtx, helper_kind: FXHelperKind) -> Span {
        fctx.get_helper_span(helper_kind)
            .or_else(|| fctx.fieldx_attr_span().as_ref().copied())
            .or_else(|| self.ctx().args().get_helper_span(helper_kind))
            .unwrap_or_else(|| Span::call_site())
    }

    fn helper_name(&self, fctx: &FXFieldCtx, helper_kind: FXHelperKind) -> darling::Result<Ident> {
        let args = self.ctx().args();
        let helper_span = fctx
            .get_helper_span(helper_kind)
            .or_else(|| args.get_helper_span(helper_kind))
            .unwrap_or_else(|| Span::call_site());

        if let Some(ref h) = fctx.get_helper(helper_kind) {
            if let Some(ref name) = h.name() {
                if !name.is_empty() {
                    return Ok(format_ident!("{}", name, span = helper_span));
                }
            }
        }

        #[cfg(not(feature = "diagnostics"))]
        let mut helper_base_name = fctx.helper_base_name()?;

        #[cfg(feature = "diagnostics")]
        let mut helper_base_name = fctx.helper_base_name().map_err(|err| {
            err.note(format!(
                "Field name is required for generating '{}' helper.",
                helper_kind.to_string()
            ))
        })?;

        // Make items, generated for for a helper, point back at the helper declaration.
        helper_base_name.set_span(helper_span);

        let args_helper = self.ctx().args().get_helper(helper_kind);
        let prefix = args_helper
            .and_then(|h| h.name())
            .or_else(|| helper_kind.default_prefix())
            .unwrap_or("");
        let suffix = helper_kind.default_suffix().unwrap_or("");

        Ok(format_ident!(
            "{}{}{}",
            prefix,
            helper_base_name,
            suffix,
            span = helper_span
        ))
    }

    fn helper_name_tok(&self, fctx: &FXFieldCtx, helper_kind: FXHelperKind) -> darling::Result<TokenStream> {
        Ok(self.helper_name(fctx, helper_kind)?.to_token_stream())
    }

    #[inline]
    fn accessor_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::Accessor)
    }

    #[inline]
    fn accessor_mut_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::AccessorMut)
    }

    #[inline]
    fn lazy_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::Lazy)
    }

    #[inline]
    fn setter_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::Setter)
    }

    #[inline]
    fn clearer_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::Clearer)
    }

    #[inline]
    fn predicate_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::Predicate)
    }

    #[inline]
    fn writer_name(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        self.helper_name_tok(fctx, FXHelperKind::Writer)
    }

    fn attributes_fn<'s>(
        &'s self,
        fctx: &'s FXFieldCtx,
        helper_kind: FXHelperKind,
        inlining: FXInlining,
    ) -> Option<TokenStream> {
        let span = fctx.helper_span(helper_kind);
        let attrs = fctx
            .get_helper(helper_kind)
            .and_then(|h| h.attributes_fn())
            .or_else(|| fctx.attributes_fn().as_ref())
            .or_else(|| {
                self.ctx()
                    .args()
                    .get_helper(helper_kind)
                    .and_then(|h| h.attributes_fn())
            });

        match inlining {
            FXInlining::Default => attrs.map(|a| quote![#a]),
            FXInlining::Inline => Some(quote_spanned![span=> #[inline] #attrs]),
            FXInlining::Always => Some(quote_spanned![span=> #[inline(always)] #attrs]),
        }
    }

    fn generic_params(&self) -> TokenStream {
        let generic_idents = self.ctx().input().generic_param_idents();

        if generic_idents.len() > 0 {
            quote![< #( #generic_idents ),* >]
        }
        else {
            quote![]
        }
    }

    fn maybe_ref_counted<TT: ToTokens>(&self, ty: &TT) -> TokenStream {
        let ctx = self.ctx();
        if ctx.args().is_ref_counted() {
            let span = ctx.args().rc().span();
            let (rc_type, _) = self.ref_count_types();
            return quote_spanned![span=> #rc_type<#ty>];
        }

        ty.to_token_stream()
    }

    fn maybe_ref_counted_create<NT: ToTokens, IT: ToTokens>(
        &self,
        self_name: &NT,
        struct_init: &IT,
        init: Option<syn::Ident>,
    ) -> TokenStream {
        let ctx = self.ctx();
        let args = ctx.args();
        let post_constuct = if let Some(init) = init {
            let shortcut = if ctx.build_has_error_type() {
                quote![?]
            }
            else {
                quote![]
            };
            quote![.#init() #shortcut]
        }
        else {
            quote![]
        };

        if args.is_ref_counted() {
            let (rc_type, _) = self.ref_count_types();
            let myself_field = ctx.myself_field();
            quote![
                #rc_type::new_cyclic(
                    |me| {
                        #self_name {
                            #myself_field: me.clone(),
                            #struct_init
                        }
                        #post_constuct
                    }
                )
            ]
        }
        else {
            quote![
                #self_name {
                    #struct_init
                }
                #post_constuct
            ]
        }
    }

    fn maybe_optional<TT: ToTokens>(&self, fctx: &FXFieldCtx, ty: TT) -> TokenStream {
        if fctx.is_optional() {
            quote![::std::option::Option<#ty>]
        }
        else {
            ty.to_token_stream()
        }
    }

    fn field_decl(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let attributes = fctx.all_attrs();
        let vis = fctx.vis();

        let ty_tok = if fctx.is_skipped() {
            fctx.ty_tok()
        }
        else {
            self.type_tokens(&fctx)?
        };
        // No check for None is needed because we're only applying to named structs.
        let ident = fctx.ident_tok();

        self.ctx().add_field_decl(quote_spanned! [*fctx.span()=>
            #( #attributes )*
            #vis #ident: #ty_tok
        ]);

        Ok(())
    }

    fn field_methods(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        if !fctx.is_skipped() {
            let ctx = self.ctx();
            ctx.add_method_decl(self.field_accessor(&fctx)?);
            ctx.add_method_decl(self.field_accessor_mut(&fctx)?);
            ctx.add_method_decl(self.field_reader(&fctx)?);
            ctx.add_method_decl(self.field_writer(&fctx)?);
            ctx.add_method_decl(self.field_setter(&fctx)?);
            ctx.add_method_decl(self.field_clearer(&fctx)?);
            ctx.add_method_decl(self.field_predicate(&fctx)?);
            ctx.add_method_decl(self.field_lazy_builder_wrapper(&fctx)?);
            if ctx.needs_builder_struct() {
                ctx.add_builder_decl(self.field_builder(&fctx)?);
                ctx.add_builder_field_decl(self.field_builder_field(&fctx)?);
                ctx.add_builder_field_ident(fctx.ident().clone());
            }
        }

        Ok(())
    }

    fn field_default(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let def_tok = if fctx.is_skipped() {
            let span = fctx.field().skip().span();
            fctx.default_value()
                .unwrap_or(quote_spanned! {span=> ::std::default::Default::default() })
        }
        else {
            self.field_default_wrap(fctx)?
        };
        let ident = fctx.ident_tok();
        self.ctx().add_defaults_decl(quote! [ #ident: #def_tok ]);
        Ok(())
    }

    fn field_default_value(&self, fctx: &FXFieldCtx) -> FXValueRepr<TokenStream> {
        let field = fctx.field();

        if let Some(def_meta) = fctx.default_value() {
            let span = def_meta.span();

            FXValueRepr::Versatile(quote_spanned! [span=> #def_meta ])
        }
        else if fctx.is_lazy() || fctx.is_optional() {
            FXValueRepr::None
        }
        else {
            FXValueRepr::Exact(quote_spanned! [field.span()=> ::std::default::Default::default() ])
        }
    }

    fn field_builder(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        Ok(if fctx.forced_builder() || fctx.needs_builder() {
            let mut mc = MethodConstructor::new(self.helper_name(fctx, FXHelperKind::Builder)?);
            let span = fctx.helper_span(FXHelperKind::Builder);
            let ident = fctx.ident_tok();
            let (val_type, gen_params, into_tok) = self.into_toks(fctx, fctx.is_builder_into());

            mc.set_span(span);
            mc.set_vis(fctx.builder_method_visibility());
            mc.maybe_add_attribute(self.attributes_fn(fctx, FXHelperKind::Builder, FXInlining::Always));
            mc.maybe_add_generic(gen_params);
            mc.set_self_mut(true);
            mc.add_param(quote_spanned! {span=> value: #val_type});
            mc.set_self_borrow(false);
            mc.set_ret_type(quote_spanned! {span=> Self});
            mc.add_statement(quote_spanned! {span=> self.#ident = ::std::option::Option::Some(value #into_tok);});
            mc.set_ret_stmt(quote_spanned! {span=> self});

            mc.into_method()
        }
        else {
            quote![]
        })
    }

    fn field_builder_field(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let ident = fctx.ident_tok();
        let span = *fctx.span();
        let ty = fctx.ty().to_token_stream();
        let attributes = fctx.builder().as_ref().and_then(|b| b.attributes());
        // Precautionary measure as this kind of fields are unlikely to be read. Yet, some of them may affect validity
        // of the builder like, for example, when they may refer to generic lifetimes.
        let allow_attr = if !fctx.forced_builder() && !fctx.needs_builder() {
            quote![#[allow(dead_code)]]
        }
        else {
            quote![]
        };
        Ok(quote_spanned![span=> #attributes #allow_attr #ident: ::std::option::Option<#ty>])
    }

    fn field_builder_value_required(&self, fctx: &FXFieldCtx) {
        if fctx.is_builder_required()
            || (fctx.needs_builder() && !(fctx.is_lazy() || fctx.is_optional() || fctx.has_default_value()))
        {
            let field_ident = fctx.ident();
            let field_name = field_ident.to_string();
            let span = self.helper_span(fctx, FXHelperKind::Builder);
            let ctx = self.ctx();
            let field_set_error = if let Some(variant) = ctx.builder_error_variant() {
                // If variant is explicitly specified then use it.
                quote_spanned! {span=> #variant(#field_name.into())}
            }
            else if ctx.builder_error_type().is_some() {
                // If there is  no variant but custom error type is requested then we expect that custom type to
                // implement From<FieldXError> trait.
                quote_spanned! {span=>
                    ::std::convert::Into::into(
                        ::fieldx::error::FieldXError::uninitialized_field(#field_name.into())
                    )
                }
            }
            else {
                quote_spanned! {span =>
                    ::fieldx::error::FieldXError::uninitialized_field(#field_name.into())
                }
            };
            fctx.set_builder_checker(quote_spanned![span=>
                if self.#field_ident.is_none() {
                    return ::std::result::Result::Err(#field_set_error)
                }
            ]);
        }
    }

    fn field_builder_value_for_set(&self, fctx: &FXFieldCtx, field_ident: &TokenStream, span: &Span) -> TokenStream {
        let ctx = self.ctx();
        let alternative = if fctx.has_default_value() {
            self.field_default_wrap(fctx).map_or_else(
                |e| {
                    ctx.push_error(e);
                    None
                },
                |tt| Some(tt),
            )
        }
        else if fctx.is_optional() && !fctx.is_builder_required() {
            self.field_value_wrap(
                fctx,
                FXValueRepr::Exact(quote_spanned![*span=> ::std::option::Option::None]),
            )
            .map_or_else(
                |e| {
                    ctx.push_error(e);
                    None
                },
                |tt| Some(tt),
            )
        }
        else {
            None
        };

        if let Some(alternative) = alternative {
            let manual_wrapped =
                self.ok_or_empty(self.field_value_wrap(fctx, FXValueRepr::Versatile(quote![field_manual_value])));
            quote_spanned![*span=>
                if let ::std::option::Option::Some(field_manual_value) = self.#field_ident.take() {
                    #manual_wrapped
                }
                else {
                    #alternative
                }
            ]
        }
        else {
            // If no alternative init path provided then we just unwrap. It'd be either totally safe if builder checker
            // is set for this field, or won't be ever run because of an earlier error in this method.
            let value_wrapped = self.ok_or_empty(
                self.field_value_wrap(fctx, FXValueRepr::Versatile(quote![self.#field_ident.take().unwrap()])),
            );
            quote_spanned![*span=> #value_wrapped ]
        }
    }

    fn fixup_self_type(&self, tokens: TokenStream) -> TokenStream {
        let ctx = self.ctx();
        let span = tokens.span();
        let mut fixed_tokens = TokenStream::new();
        let struct_ident = ctx.input_ident();
        let (_, generics, _) = ctx.input().generics().split_for_impl();

        for t in tokens.into_iter() {
            match t {
                TokenTree::Ident(ref ident) => {
                    if ident.to_string() == "Self" {
                        fixed_tokens.extend(quote_spanned![span=> <#struct_ident #generics>]);
                    }
                    else {
                        fixed_tokens.extend(t.to_token_stream());
                    }
                }
                TokenTree::Group(ref group) => {
                    let mut group = proc_macro2::Group::new(group.delimiter(), self.fixup_self_type(group.stream()));
                    group.set_span(span);
                    fixed_tokens.extend(TokenTree::Group(group).to_token_stream())
                }
                _ => fixed_tokens.extend(t.to_token_stream()),
            }
        }

        quote_spanned![span=> #fixed_tokens]
    }

    // TokenStreams used to produce methods with Into support.
    fn into_toks(&self, fctx: &FXFieldCtx, use_into: bool) -> (TokenStream, Option<TokenStream>, Option<TokenStream>) {
        let ty = fctx.ty();
        if use_into {
            (
                quote![FXVALINTO],
                Some(quote![FXVALINTO: ::std::convert::Into<#ty>]),
                Some(quote![.into()]),
            )
        }
        else {
            (quote![#ty], None, None)
        }
    }

    fn simple_field_build_setter(&self, fctx: &FXFieldCtx, field_ident: &TokenStream, span: &Span) -> TokenStream {
        let set_toks = self.field_builder_value_for_set(fctx, field_ident, span);

        quote_spanned![*span=> #field_ident: #set_toks ]
    }

    fn maybe_ref_counted_self(&self, fctx: &FXFieldCtx, mc: &mut MethodConstructor) {
        if mc.self_rc_ident().is_some() {
            // Already set, no need to do it again
            return;
        }
        let ctx = self.ctx();
        let args = ctx.args();
        if args.is_ref_counted() {
            let (myself_method, _) = ctx.myself_names().unwrap();
            let span = args.rc_span();
            let self_rc = format_ident!("__fx_self_rc", span = span);
            let self_ident = mc.self_ident();
            let expect_msg = format!("Can't obtain weak reference to myself for field {}", fctx.ident());
            mc.add_statement(quote_spanned! {span=> let #self_rc = #self_ident.#myself_method().expect(#expect_msg);});
            mc.set_self_rc_ident(Some(self_rc.to_token_stream()));
        }
    }

    #[inline]
    fn builder_return_type(&self) -> TokenStream {
        let ctx = self.ctx();
        let builder_ident = ctx.input_ident();
        let generic_params = ctx.struct_generic_params();
        quote![#builder_ident #generic_params]
    }

    fn fallible_return_type<TT>(&self, fctx: &FXFieldCtx, ty: TT) -> darling::Result<TokenStream>
    where
        TT: ToTokens,
    {
        let ty = ty.to_token_stream();
        Ok(if fctx.is_fallible() {
            let error_type = fctx.fallible_error()?;
            let span = fctx.fallible_span();
            quote_spanned! {span=> ::std::result::Result<#ty, #error_type>}
        }
        else {
            quote![#ty]
        })
    }
}
