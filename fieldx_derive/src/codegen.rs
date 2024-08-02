mod context;
mod nonsync;
#[cfg(feature = "serde")]
mod serde;
mod sync;

#[cfg(feature = "serde")]
pub(crate) use self::serde::FXCGenSerde;
use crate::{fields::FXField, helper::*, util::args::FXSArgs, FXInputReceiver};
use context::{FXCodeGenCtx, FXFieldCtx};
use darling::ast::NestedMeta;
use enum_dispatch::enum_dispatch;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};
use syn::{spanned::Spanned, Ident, Lit};

#[derive(PartialEq)]
pub(crate) enum FXValueRepr<T> {
    None,
    Exact(T),
    Versatile(T),
}

impl<T> FXValueRepr<T> {
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, FXValueRepr::None)
    }

    pub(crate) fn expect(self, msg: &str) -> T {
        match self {
            FXValueRepr::None => panic!("{}", msg),
            FXValueRepr::Exact(v) => v,
            FXValueRepr::Versatile(v) => v,
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn unwrap_or(self, default: T) -> T {
        match self {
            FXValueRepr::None => default,
            FXValueRepr::Exact(v) => v,
            FXValueRepr::Versatile(v) => v,
        }
    }

    pub(crate) fn map<U>(self, mapper: impl FnOnce(T) -> U) -> FXValueRepr<U> {
        match self {
            FXValueRepr::None => FXValueRepr::None,
            FXValueRepr::Exact(v) => FXValueRepr::Exact(mapper(v)),
            FXValueRepr::Versatile(v) => FXValueRepr::Versatile(mapper(v)),
        }
    }
}

// Methods that are related to the current context if first place.
#[enum_dispatch]
pub(crate) trait FXCGenContextual<'f> {
    fn ctx(&self) -> &FXCodeGenCtx;

    // Actual code producers
    fn field_accessor(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_accessor_mut(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_builder_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_reader(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_writer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_setter(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_clearer(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_predicate(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_value_wrap(&self, fctx: &FXFieldCtx, value: FXValueRepr<TokenStream>) -> darling::Result<TokenStream>;
    fn field_default_wrap(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    fn field_lazy_initializer(
        &self,
        fctx: &FXFieldCtx,
        self_ident: Option<TokenStream>,
    ) -> darling::Result<TokenStream>;
    #[cfg(feature = "serde")]
    // How to move field from shadow struct
    fn field_from_shadow(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;
    #[cfg(feature = "serde")]
    // How to move field from the struct itself
    fn field_from_struct(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream>;

    fn fxstruct_trait(&self) -> TokenStream;

    fn add_field_decl(&self, field: TokenStream);
    fn add_defaults_decl(&self, defaults: TokenStream);
    fn add_method_decl(&self, method: TokenStream);
    fn add_builder_decl(&self, builder_method: TokenStream);
    fn add_builder_field_decl(&self, builder_field: TokenStream);
    fn add_builder_field_ident(&self, fctx: syn::Ident);
    fn add_for_copy_trait_check(&self, fctx: &FXFieldCtx);
    #[cfg(feature = "serde")]
    fn add_shadow_field_decl(&self, field: TokenStream);
    #[cfg(feature = "serde")]
    fn add_shadow_default_decl(&self, field: TokenStream);

    fn type_tokens<'s>(&'s self, fctx: &'s FXFieldCtx) -> &'s TokenStream;
    fn ref_count_type(&self) -> TokenStream;
    fn copyable_types(&self) -> Ref<Vec<syn::Type>>;
    #[cfg(feature = "serde")]
    fn shadow_fields(&self) -> Ref<Vec<TokenStream>>;
    #[cfg(feature = "serde")]
    fn shadow_defaults(&self) -> Ref<Vec<TokenStream>>;

    fn field_ctx_table(&'f self) -> Ref<HashMap<Ident, FXFieldCtx<'f>>>;
    fn field_ctx_table_mut(&'f self) -> RefMut<HashMap<Ident, FXFieldCtx<'f>>>;
    fn builder_field_ident(&self) -> &RefCell<Vec<syn::Ident>>;
    fn methods_combined(&self) -> TokenStream;
    fn defaults_combined(&self) -> TokenStream;
    fn builder_fields_combined(&self) -> TokenStream;
    fn builders_combined(&self) -> TokenStream;
    fn struct_fields(&self) -> Ref<Vec<TokenStream>>;

    #[inline]
    fn needs_builder_struct(&self) -> bool {
        self.ctx().needs_builder_struct().unwrap_or(false)
    }

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

    fn ident_field_ctx(&'f self, field_ident: &syn::Ident) -> darling::Result<Ref<FXFieldCtx<'f>>> {
        let fctx_table = self.field_ctx_table();
        Ref::filter_map(fctx_table, |ft| ft.get(field_ident))
            .map_err(|_| darling::Error::custom(format!("No context found for field `{}`", field_ident)))
    }

    fn field_ctx(&'f self, field: &'f FXField) -> darling::Result<Ref<FXFieldCtx<'f>>> {
        let field_ident = field.ident()?;
        {
            let mut fctx_table = self.field_ctx_table_mut();
            if !fctx_table.contains_key(&field_ident) {
                let _ = fctx_table.insert(field_ident.clone(), <FXFieldCtx<'f>>::new(field, self.ctx()));
            }
        }
        self.ident_field_ctx(&field_ident)
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

    fn attributes_fn<'s>(&'s self, fctx: &'s FXFieldCtx, helper_kind: FXHelperKind) -> Option<&'s FXAttributes> {
        fctx.get_helper(helper_kind)
            .and_then(|h| h.attributes_fn())
            .or_else(|| fctx.attributes_fn().as_ref())
            .or_else(|| {
                self.ctx()
                    .args()
                    .get_helper(helper_kind)
                    .and_then(|h| h.attributes_fn())
            })
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

    fn maybe_ref_counted<TT: ToTokens, ET: ToTokens>(
        &self,
        ctx: &FXCodeGenCtx,
        ty: &TT,
        expr: &ET,
    ) -> (TokenStream, TokenStream) {
        if ctx.args().is_ref_counted() {
            let span = ctx.args().rc().span();
            let rc_type = self.ref_count_type();
            return (
                quote_spanned![span=> #rc_type<#ty>],
                quote_spanned![span=> #rc_type::new(#expr)],
            );
        }

        (ty.to_token_stream(), expr.to_token_stream())
    }

    fn field_default_value(&self, fctx: &FXFieldCtx) -> darling::Result<FXValueRepr<TokenStream>> {
        let field = fctx.field();

        Ok(if let Some(def_meta) = fctx.default_value() {
            let mut is_str = false;
            let span = def_meta.span();

            if let NestedMeta::Lit(Lit::Str(_lit_val)) = def_meta {
                is_str = true;
            }

            FXValueRepr::Versatile(if is_str {
                quote_spanned! [span=> ::std::string::String::from(#def_meta) ]
            }
            else {
                quote_spanned! [span=> #def_meta ]
            })
        }
        else if fctx.is_lazy() || fctx.is_optional() {
            FXValueRepr::None
        }
        else {
            FXValueRepr::Exact(quote_spanned! [field.span()=> ::std::default::Default::default() ])
        })
    }

    fn derive_toks(&self, traits: &[TokenStream]) -> TokenStream {
        if traits.len() > 0 {
            quote!(#[derive(#( #traits ),*)])
        }
        else {
            quote![]
        }
    }

    fn fixup_self_type(&self, tokens: TokenStream) -> TokenStream {
        let ctx = self.ctx();
        let span = tokens.span();
        let mut fixed_tokens = TokenStream::new();
        let struct_ident = ctx.input_ident();
        let generics = ctx.input().generics();

        for t in tokens.into_iter() {
            match t {
                TokenTree::Ident(ref ident) => {
                    if ident.to_string() == "Self" {
                        fixed_tokens.extend(quote![<#struct_ident #generics>]);
                    }
                    else {
                        fixed_tokens.extend(t.to_token_stream());
                    }
                }
                TokenTree::Group(ref group) => fixed_tokens.extend(
                    TokenTree::Group(proc_macro2::Group::new(
                        group.delimiter(),
                        self.fixup_self_type(group.stream()),
                    ))
                    .to_token_stream(),
                ),
                _ => fixed_tokens.extend(t.to_token_stream()),
            }
        }

        quote_spanned![span=> #fixed_tokens]
    }
}

pub(crate) trait FXCGen<'f>: FXCGenContextual<'f> {
    // TokenStreams used to produce methods with Into support.
    fn into_toks(&self, fctx: &FXFieldCtx, use_into: bool) -> (TokenStream, TokenStream, TokenStream) {
        let ty = fctx.ty();
        if use_into {
            (
                quote![<FXVALINTO: ::std::convert::Into<#ty>>],
                quote![FXVALINTO],
                quote![.into()],
            )
        }
        else {
            (quote![], quote![#ty], quote![])
        }
    }

    fn input_type_toks(&self) -> TokenStream {
        let ident = self.ctx().input_ident();
        let generic_params = self.generic_params();
        quote::quote! {
            #ident #generic_params
        }
    }

    fn field_decl(&self, fctx: &FXFieldCtx<'f>) {
        let attributes = fctx.all_attrs();
        let vis = fctx.vis();

        let ty_tok = self.type_tokens(&fctx);
        // No check for None is needed because we're only applying to named structs.
        let ident = fctx.ident_tok();

        self.add_field_decl(quote_spanned! [*fctx.span()=>
            #( #attributes )*
            #vis #ident: #ty_tok
        ]);
    }

    fn field_methods(&self, fctx: &FXFieldCtx<'f>) -> darling::Result<()> {
        if !fctx.is_skipped() {
            self.add_method_decl(self.field_accessor(&fctx)?);
            self.add_method_decl(self.field_accessor_mut(&fctx)?);
            self.add_method_decl(self.field_reader(&fctx)?);
            self.add_method_decl(self.field_writer(&fctx)?);
            self.add_method_decl(self.field_setter(&fctx)?);
            self.add_method_decl(self.field_clearer(&fctx)?);
            self.add_method_decl(self.field_predicate(&fctx)?);
            if self.needs_builder_struct() {
                self.add_builder_decl(self.field_builder(&fctx)?);
                self.add_builder_field_decl(self.field_builder_field(&fctx)?);
                self.add_builder_field_ident(fctx.ident()?.clone());
            }
        }

        Ok(())
    }

    fn ensure_builder_is_needed(&self) {
        let ctx = self.ctx();
        // If builder requirement is not set explicitly with fxstruct attribute then check out if any field is asking
        // for it.
        if ctx.needs_builder_struct().is_none() {
            for field in self.input().fields() {
                if let Some(needs) = field.needs_builder() {
                    if needs {
                        self.ctx().require_builder();
                    }
                }
            }
        }
    }

    fn prepare_field(&'f self, fctx: Ref<FXFieldCtx<'f>>) -> darling::Result<()> {
        if fctx.needs_accessor() && fctx.is_copy() {
            self.add_for_copy_trait_check(&fctx);
        }

        self.field_default(&fctx)?;
        self.field_methods(&fctx)?;

        // Has to always be the last here as it may use attributes added by the previous methods.
        self.field_decl(&fctx);

        Ok(())
    }

    fn prepare_struct(&'f self) {
        self.ensure_builder_is_needed();

        for field in self.input().fields() {
            let Ok(fctx) = self.field_ctx(field)
            else {
                continue;
            };
            self.ok_or_record(self.prepare_field(fctx));
        }
    }

    fn rewrite_struct(&'f self) {
        self.struct_extras();

        if self.needs_builder_struct() {
            let builder_ident = self.builder_ident();
            let generic_params = self.generic_params();
            let vis = self.ctx().input().vis();
            self.add_method_decl(quote![
                #[inline]
                #vis fn builder() -> #builder_ident #generic_params {
                    #builder_ident::default()
                }
            ])
        }
    }

    fn field_default(&self, fctx: &FXFieldCtx) -> darling::Result<()> {
        let def_tok = self.field_default_wrap(fctx)?;
        let ident = fctx.ident_tok();
        self.add_defaults_decl(quote! [ #ident: #def_tok ]);
        Ok(())
    }

    fn field_builder_value_for_set(&self, fctx: &FXFieldCtx, field_ident: &TokenStream, span: &Span) -> TokenStream {
        let field_name = field_ident.to_string();
        let alternative = if fctx.has_default_value() {
            self.ok_or_empty(self.field_default_wrap(fctx))
        }
        else if fctx.is_optional() {
            self.ok_or_empty(self.field_value_wrap(
                fctx,
                FXValueRepr::Exact(quote_spanned![*span=> ::std::option::Option::None]),
            ))
        }
        else {
            quote_spanned![*span=>
                return ::std::result::Result::Err(
                    ::std::convert::Into::into(
                        ::fieldx::errors::FieldXError::uninitialized_field(#field_name.into()) )
                )
            ]
        };

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

    fn simple_field_build_setter(&self, fctx: &FXFieldCtx, field_ident: &TokenStream, span: &Span) -> TokenStream {
        let set_toks = self.field_builder_value_for_set(fctx, field_ident, span);

        quote_spanned![*span=> #field_ident: #set_toks ]
    }

    fn default_impl(&self) -> TokenStream {
        let ctx = self.ctx();

        if !ctx.needs_default() {
            return quote![];
        }

        let defaults = self.defaults_combined();
        let ident = ctx.input().ident();
        let generics = ctx.input().generics();
        let where_clause = &generics.where_clause;

        if !defaults.is_empty() {
            quote! [
                impl #generics Default for #ident #generics #where_clause {
                    fn default() -> Self {
                        Self { #defaults }
                    }
                }
            ]
        }
        else {
            // It's already empty, what sense in allocating another copy?
            defaults
        }
    }

    fn builder_ident(&self) -> TokenStream {
        let ctx = self.ctx();
        let ident = ctx.input_ident();
        format_ident!("{}{}", ident, "Builder", span = ctx.helper_span(FXHelperKind::Builder)).to_token_stream()
    }

    fn builder_field_ctxs(&'f self) -> Vec<darling::Result<Ref<FXFieldCtx<'f>>>> {
        let builder_field_idents = self.builder_field_ident().borrow();
        builder_field_idents
            .iter()
            .map(|ident| self.ident_field_ctx(&ident))
            .collect()
    }

    fn field_builder_field(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if fctx.needs_builder() {
            let ident = fctx.ident_tok();
            let span = *fctx.span();
            let ty = fctx.ty();
            let attributes = fctx.builder().as_ref().and_then(|b| b.attributes());
            if fctx.is_ignorable() {
                Ok(quote_spanned![span=> #attributes #ident: #ty])
            }
            else {
                Ok(quote_spanned![span=> #attributes #ident: ::std::option::Option<#ty>])
            }
        }
        else {
            Ok(quote![])
        }
    }

    fn field_builder(&self, fctx: &FXFieldCtx) -> darling::Result<TokenStream> {
        if !fctx.is_ignorable() && fctx.needs_builder() {
            let ident = fctx.ident_tok();
            let mut builder_name = self.helper_name(fctx, FXHelperKind::Builder)?;
            // .builder_name(fctx)?;
            // .unwrap_or(format_ident!("{}", fctx.helper_base_name().expect("Field name")).to_token_stream());
            let span = fctx.helper_span(FXHelperKind::Builder);
            let (gen_params, val_type, into_tok) = self.into_toks(fctx, fctx.is_builder_into());
            let attributes = self.attributes_fn(fctx, FXHelperKind::Builder);
            builder_name.set_span(span);
            Ok(quote_spanned![span=>
                #attributes
                pub fn #builder_name #gen_params(&mut self, value: #val_type) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(value #into_tok);
                    self
                }
            ])
        }
        else {
            Ok(quote![])
        }
    }

    fn builder_struct(&'f self) -> TokenStream {
        if self.needs_builder_struct() {
            let ctx = self.ctx();
            let args = ctx.args();
            let builder_fields = self.builder_fields_combined();
            let builder_impl = self.builder_impl();
            let generics = ctx.input().generics();
            let where_clause = &generics.where_clause;
            let span = ctx.helper_span(FXHelperKind::Builder);
            let vis = self.builder_struct_visibility();
            let attributes = args.builder_attributes();
            let traits = vec![quote![Default]];
            let derive_attr = self.derive_toks(&traits);
            let builder_ident = self.builder_ident();

            quote_spanned![span=>
                #derive_attr
                #attributes
                #vis struct #builder_ident #generics
                #where_clause
                {
                    #builder_fields
                }

                #builder_impl
            ]
        }
        else {
            quote![]
        }
    }

    #[inline]
    fn builder_return_type(&self) -> TokenStream {
        let builder_ident = self.ctx().input_ident();
        let generic_params = self.generic_params();
        quote![#builder_ident #generic_params]
    }

    fn builder_struct_visibility(&self) -> TokenStream {
        let ctx = self.ctx();
        ctx.args()
            .get_helper(FXHelperKind::Builder)
            .and_then(|builder| builder.public_mode().map(|pm| pm.to_token_stream()))
            .or_else(|| Some(ctx.input().vis().to_token_stream()))
            .unwrap()
    }

    fn builder_impl(&'f self) -> TokenStream {
        let ctx = self.ctx();
        let vis = self.builder_struct_visibility();
        let builder_ident = self.builder_ident();
        let builders = self.builders_combined();
        let input_ident = ctx.input_ident();
        let generics = ctx.input().generics();
        let where_clause = &generics.where_clause;
        let generic_params = self.generic_params();
        let attributes = ctx.args().builder_impl_attributes();

        let mut field_setters = Vec::<TokenStream>::new();
        let mut use_default = false;
        for fctx in self.builder_field_ctxs() {
            if let Ok(fctx) = fctx {
                let fsetter = self.ok_or_empty(self.field_builder_setter(&fctx));
                if fsetter.is_empty() {
                    use_default = true;
                }
                else {
                    field_setters.push(fsetter);
                }
            }
            else {
                self.ctx().push_error(fctx.unwrap_err());
            }
        }

        let default_initializer = if use_default {
            quote![..::std::default::Default::default()]
        }
        else {
            quote![]
        };

        let (builder_return_type, construction) = self.maybe_ref_counted(
            ctx,
            &self.builder_return_type(),
            &quote![
                {
                    #input_ident {
                        #(#field_setters,)*
                        #default_initializer
                    }
                }
            ],
        );

        quote![
            #attributes
            impl #generics #builder_ident #generic_params
            #where_clause
            {
                #builders
                #vis fn build(&mut self) -> ::std::result::Result<#builder_return_type, ::fieldx::errors::FieldXError> {
                    Ok(#construction)
                }
            }
        ]
    }

    fn struct_extras(&'f self) {
        let ctx = self.ctx();
        // let initializers = self.initializers_combined(); // self.initializer_toks.borrow_mut();
        let generics = ctx.input().generics();
        let generic_params = self.generic_params();
        let input = ctx.input_ident();
        let where_clause = &generics.where_clause;
        let struct_trait = self.fxstruct_trait();

        ctx.tokens_extend(quote![
            impl #generics #struct_trait for #input #generic_params #where_clause {}
        ]);

        if ctx.needs_default() {
            // Generate fn new()
            let new_name = if ctx.args().needs_new() {
                quote![new]
            }
            else {
                quote![__fieldx_new]
            };

            let (return_type, body) = self.maybe_ref_counted(ctx, &quote![Self], &quote![Self::default()]);

            self.add_method_decl(quote![
                #[inline]
                pub fn #new_name() -> #return_type {
                    #body
                }
            ]);
        }
    }

    fn finalize(&'f self) -> TokenStream {
        let ctx = self.ctx();

        let &FXInputReceiver {
            ref vis,
            ref ident,
            ref generics,
            ..
        } = ctx.input();

        // ctx.add_attr(self.derive_toks(&self.derive_traits()));

        let attributes = ctx.all_attrs();
        let attributes_impl = ctx.args().attributes_impl().as_ref();
        let methods = self.methods_combined();
        let fields = self.struct_fields();
        let default = self.default_impl();
        let builder_struct = self.builder_struct();
        let where_clause = &generics.where_clause;
        let generic_params = self.generic_params();

        let copyables = self.copyable_types();
        let copyable_validation = if !copyables.is_empty() {
            let copyables: Vec<TokenStream> = copyables.iter().map(|ct| ct.to_token_stream()).collect();
            Some(quote![
                #[allow(dead_code)]
                fn __fieldx_copy_validation() {
                    fn field_implements_copy<T: ?Sized + Copy>() {}
                    #( field_implements_copy::<#copyables>(); )*
                }
            ])
        }
        else {
            None
        };

        ctx.tokens_extend(quote! [
            use ::fieldx::traits::*;

            #( #attributes )*
            #vis struct #ident #generics
            #where_clause
            {
                #( #fields ),*
            }

            impl #generics FXStruct for #ident #generic_params #where_clause {}

            #attributes_impl
            impl #generics #ident #generics #where_clause {
                #methods
                #copyable_validation
            }

            #default

            #builder_struct
        ]);
        ctx.finalize()
    }
}

// FieldX Code Generator – FXCG
#[enum_dispatch(FXCGenContextual)]
enum FXCG<'f> {
    NonSync(nonsync::FXCodeGen<'f>),
    Sync(sync::FXCodeGen<'f>),
}

impl<'f, T> FXCGen<'f> for T where T: FXCGenContextual<'f> {}

pub struct FXRewriter<'f> {
    generator: FXCG<'f>,
}

impl<'f> FXRewriter<'f> {
    pub fn new(input: FXInputReceiver, args: FXSArgs) -> Self {
        let ctx = FXCodeGenCtx::new(input, args);

        let generator: FXCG = if ctx.args().is_sync() {
            FXCG::Sync(sync::FXCodeGen::new(ctx))
        }
        else {
            FXCG::NonSync(nonsync::FXCodeGen::new(ctx))
        };

        Self { generator }
    }

    pub fn rewrite(&'f mut self) -> TokenStream {
        self.generator.prepare_struct();
        #[cfg(feature = "serde")]
        self.generator.serde_prepare_struct();
        self.generator.rewrite_struct();
        #[cfg(feature = "serde")]
        self.generator.serde_rewrite_struct();
        self.generator.finalize()
    }
}
