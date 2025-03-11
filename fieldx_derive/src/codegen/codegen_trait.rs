use super::{
    constructor::method::MethodConstructor, FXCodeGenCtx, FXCodeGenPlain, FXCodeGenSync, FXFieldCtx, FXInlining,
    FXValueRepr,
};
use crate::{helper::*, FXInputReceiver};
use enum_dispatch::enum_dispatch;
use fieldx_aux::{FXProp, FXPropBool};
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::rc::Rc;
use syn::spanned::Spanned;

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
    fn ref_count_types(&self, span: Span) -> (TokenStream, TokenStream);
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

    fn generic_params(&self) -> TokenStream {
        let input = self.ctx().input();
        let generic_idents = input.generic_param_idents();
        let span = input.generics().span();

        if generic_idents.len() > 0 {
            quote_spanned![span=> < #( #generic_idents ),* >]
        }
        else {
            quote![]
        }
    }

    fn maybe_ref_counted<TT: ToTokens>(&self, ty: &TT) -> TokenStream {
        let ref_counted = self.ctx().arg_props().rc();
        if *ref_counted {
            let rc_span = ref_counted.final_span();
            let (rc_type, _) = self.ref_count_types(rc_span);
            return quote_spanned![rc_span=> #rc_type<#ty>];
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
        let arg_props = ctx.arg_props();
        let post_construct = if let Some(init) = init {
            let builder_has_error_type = arg_props.builder_has_error_type();
            let shortcut = if *builder_has_error_type {
                quote_spanned![builder_has_error_type.final_span()=> ?]
            }
            else {
                quote![]
            };
            quote_spanned![init.span()=> .#init() #shortcut]
        }
        else {
            quote![]
        };

        let rc = arg_props.rc();

        if *rc {
            let rc_span = rc.final_span();
            let (rc_type, _) = self.ref_count_types(rc_span);
            let myself_field = arg_props.myself_field_ident();
            quote_spanned![rc_span=>
                #rc_type::new_cyclic(
                    |me| {
                        #self_name {
                            #myself_field: me.clone(),
                            #struct_init
                        }
                        #post_construct
                    }
                )
            ]
        }
        else {
            quote_spanned![self_name.span()=>
                #self_name {
                    #struct_init
                }
                #post_construct
            ]
        }
    }

    fn maybe_optional<T: ToTokens>(&self, fctx: &FXFieldCtx, ty: T) -> TokenStream {
        let opt = fctx.optional();
        if *opt {
            quote_spanned! {opt.final_span()=> ::std::option::Option<#ty>}
        }
        else {
            ty.to_token_stream()
        }
    }

    fn field_decl(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let mut constructor = fctx.constructor();

        constructor.set_vis(fctx.vis());

        if !*fctx.skipped() {
            constructor.set_type(self.type_tokens(&fctx)?.clone());
        };
        // No check for None is needed because we're only applying to named structs.

        self.ctx().add_field_decl(constructor.to_field());

        Ok(())
    }

    fn field_methods(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        if !*fctx.skipped() {
            let ctx = self.ctx();
            ctx.add_method_decl(self.field_accessor(&fctx)?);
            ctx.add_method_decl(self.field_accessor_mut(&fctx)?);
            ctx.add_method_decl(self.field_reader(&fctx)?);
            ctx.add_method_decl(self.field_writer(&fctx)?);
            ctx.add_method_decl(self.field_setter(&fctx)?);
            ctx.add_method_decl(self.field_clearer(&fctx)?);
            ctx.add_method_decl(self.field_predicate(&fctx)?);
            ctx.add_method_decl(self.field_lazy_builder_wrapper(&fctx)?);
            if *ctx.arg_props().builder_struct() {
                ctx.add_builder_decl(self.field_builder(&fctx)?);
                ctx.add_builder_field_decl(self.field_builder_field(&fctx)?);
                ctx.add_builder_field_ident(fctx.ident().clone());
            }
        }

        Ok(())
    }

    fn field_default(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let skipped = fctx.skipped();
        let def_tok = if *skipped {
            let span = skipped.final_span();
            fctx.default_value().map_or_else(
                || quote_spanned! {span=> ::std::default::Default::default() },
                |dv| dv.to_token_stream(),
            )
        }
        else {
            self.field_default_wrap(fctx)?
        };
        let ident = fctx.ident();
        self.ctx()
            .add_defaults_decl(quote_spanned! [ident.span()=> #ident: #def_tok ]);
        Ok(())
    }

    fn field_default_value(&self, fctx: &FXFieldCtx) -> FXValueRepr<TokenStream> {
        if let Some(default_value) = fctx.default_value() {
            FXValueRepr::Versatile(default_value.to_token_stream())
        }
        else if *fctx.lazy() || *fctx.optional() {
            FXValueRepr::None
        }
        else {
            FXValueRepr::Exact(quote_spanned! [*fctx.span()=> ::std::default::Default::default() ])
        }
    }

    fn field_builder(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let builder = fctx.forced_builder().or(fctx.builder());
        Ok(if *builder {
            let mut mc = MethodConstructor::new(fctx.builder_ident());
            let span = builder.final_span();
            let ident = fctx.ident();
            let (val_type, gen_params, into_tok) = self.into_toks(fctx, fctx.builder_into());

            mc.set_span(span);
            mc.set_vis(fctx.builder_method_visibility());
            mc.add_attribute(fctx.helper_attributes_fn(FXHelperKind::Builder, FXInlining::Always, span));
            mc.maybe_add_generic(gen_params);
            mc.set_self_mut(true);
            mc.add_param(quote_spanned! {span=> value: #val_type});
            mc.set_self_borrow(false);
            mc.set_ret_type(quote_spanned! {span=> Self});
            mc.add_statement(quote_spanned! {span=> self.#ident = ::std::option::Option::Some(value #into_tok);});
            mc.set_ret_stmt(quote_spanned! {span=> self});

            mc.to_method()
        }
        else {
            quote![]
        })
    }

    fn field_builder_field(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        let ident = fctx.ident();
        let span = *fctx.span();
        let ty = fctx.ty().to_token_stream();
        let attributes = fctx.props().field_props().builder_attributes();
        // A precautionary measure as this kind of fields are unlikely to be read. However, some attributes may affect
        // the validity of the builder, like when they refer to generic lifetimes.
        // A reminder to self: this used to be `!forced_builder && !builder`.
        let not_builder = fctx.forced_builder().or(fctx.builder()).not();
        let allow_attr = if *not_builder {
            quote_spanned![not_builder.final_span()=> #[allow(dead_code)]]
        }
        else {
            quote![]
        };
        Ok(quote_spanned![span=> #attributes #allow_attr #ident: ::std::option::Option<#ty>])
    }

    // Ensure that we return an error or panic if the builder field is required but not set.
    fn field_builder_value_required(&self, fctx: &FXFieldCtx) {
        let builder_required = fctx.builder_required();
        let builder = fctx.builder();
        if *builder_required || (*builder && !(*fctx.lazy() || *fctx.optional() || fctx.has_default_value())) {
            let field_ident = fctx.ident();
            let field_name = field_ident.to_string();
            let span = builder_required.or(builder).final_span();
            let ctx = self.ctx();
            let arg_props = ctx.arg_props();

            let field_set_error = if let Some(variant) = arg_props.builder_error_variant() {
                // If variant is explicitly specified then use it.
                quote_spanned! {span=> #variant(#field_name.into())}
            }
            else {
                let mut error_create = quote_spanned! {span=>
                    ::fieldx::error::FieldXError::uninitialized_field(#field_name.into())
                };

                if arg_props.builder_error_type().is_some() {
                    // If there is  no variant but custom error type is requested then we expect that custom type to
                    // implement From<FieldXError> trait.
                    error_create = quote_spanned! {span=>
                        ::std::convert::Into::into(#error_create)
                    };
                }

                error_create
            };

            fctx.set_builder_checker(quote_spanned![span=>
                if self.#field_ident.is_none() {
                    return ::std::result::Result::Err(#field_set_error)
                }
            ]);
        }
    }

    fn field_builder_value_for_set(&self, fctx: &FXFieldCtx, field_ident: &syn::Ident, span: &Span) -> TokenStream {
        let ctx = self.ctx();
        let span = *span;
        let optional = fctx.optional();
        let builder_required = fctx.builder_required();
        let alternative = if fctx.has_default_value() {
            self.field_default_wrap(fctx).map_or_else(
                |e| {
                    ctx.push_error(e);
                    None
                },
                |tt| Some(tt),
            )
        }
        else if *optional && !*builder_required {
            self.field_value_wrap(
                fctx,
                FXValueRepr::Exact(quote_spanned![optional.final_span()=> ::std::option::Option::None]),
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
            let manual_wrapped = self.ok_or_empty(
                self.field_value_wrap(fctx, FXValueRepr::Versatile(quote_spanned![span=>field_manual_value])),
            );
            quote_spanned![span=>
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
            let value_wrapped = self.ok_or_empty(self.field_value_wrap(
                fctx,
                FXValueRepr::Versatile(quote_spanned![span=> self.#field_ident.take().unwrap()]),
            ));
            quote_spanned![span=> #value_wrapped ]
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
    fn into_toks(
        &self,
        fctx: &FXFieldCtx,
        use_into: FXProp<bool>,
    ) -> (TokenStream, Option<TokenStream>, Option<TokenStream>) {
        let ty = fctx.ty();
        if *use_into {
            let span = use_into.final_span();
            (
                quote_spanned![span=> FXVALINTO],
                Some(quote_spanned![span=> FXVALINTO: ::std::convert::Into<#ty>]),
                Some(quote_spanned![span=> .into()]),
            )
        }
        else {
            (ty.to_token_stream(), None, None)
        }
    }

    fn simple_field_build_setter(&self, fctx: &FXFieldCtx, field_ident: &syn::Ident, span: &Span) -> TokenStream {
        let set_toks = self.field_builder_value_for_set(fctx, field_ident, span);

        quote_spanned![*span=> #field_ident: #set_toks ]
    }

    fn maybe_ref_counted_self(&self, fctx: &FXFieldCtx, mc: &mut MethodConstructor) {
        if mc.self_rc_ident().is_some() {
            // Already set, no need to do it again
            return;
        }
        let ctx = self.ctx();
        let arg_props = ctx.arg_props();
        let rc = arg_props.rc();
        if *rc {
            let myself_method = arg_props.myself_name();
            let span = rc.final_span();
            let self_rc = format_ident!("__fx_self_rc", span = span);
            let self_ident = mc.self_ident();
            let expect_msg = format!("Can't obtain weak reference to myself for field {}", fctx.ident());
            mc.add_statement(quote_spanned! {span=>
                let #self_rc = #self_ident.#myself_method().expect(#expect_msg);
            });
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
        let fallible = fctx.fallible();
        Ok(if *fallible {
            let error_type = fctx.fallible_error();
            quote_spanned! {fallible.final_span()=> ::std::result::Result<#ty, #error_type>}
        }
        else {
            ty.to_token_stream()
        })
    }
}
