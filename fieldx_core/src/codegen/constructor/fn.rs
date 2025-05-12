use fieldx_aux::FXProp;
use getset::Getters;
use getset::Setters;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use syn::spanned::Spanned;

use crate::types::meta::FXToksMeta;
use crate::types::meta::FXValueFlag;

use super::tokenstream_setter;
use super::FXConstructor;

#[derive(Debug, Getters, Setters)]
#[getset(get = "pub")]
pub struct FXFnConstructor {
    name:          syn::Ident,
    associated:    bool,
    vis:           Option<TokenStream>,
    #[getset(skip)]
    is_async:      FXProp<bool>,
    // Here and below, self refers to the first parameter of the method, even if it's not actually a variant of Self.
    // As it comes in parameter list.
    #[getset(skip)]
    self_ident:    Option<TokenStream>,
    // Self type, if differrent from Self.
    self_type:     Option<TokenStream>,
    #[getset(skip)]
    self_lifetime: Option<TokenStream>,
    // Strong rc-wrapped.
    self_rc_ident: Option<TokenStream>,
    #[getset(set = "pub")]
    self_borrow:   bool,
    #[getset(set = "pub")]
    self_mut:      bool,
    #[getset(get_mut)]
    attributes:    Vec<syn::Attribute>,
    #[getset(get_mut)]
    lifetimes:     Vec<TokenStream>,
    #[getset(get_mut)]
    generics:      Vec<TokenStream>,
    #[getset(get_mut)]
    // I call it `bounds` because it doesn't contain `where` keyword.
    where_bounds: Vec<TokenStream>,
    #[getset(get_mut)]
    params:        Vec<TokenStream>,
    #[getset(get_mut)]
    body:          Vec<TokenStream>,
    ret_stmt:      Option<TokenStream>,
    ret_type:      Option<TokenStream>,
    #[getset(set = "pub")]
    ret_mut:       bool,
    #[getset(skip)]
    span:          Option<Span>,
}

impl FXFnConstructor {
    tokenstream_setter! {
        vis, self_type, ret_type, ret_stmt
    }

    pub fn new(name: syn::Ident) -> Self {
        Self {
            name,
            associated: false,
            vis: None,
            self_borrow: true,
            self_mut: false,
            self_ident: None,
            self_type: None,
            self_rc_ident: None,
            self_lifetime: None,
            is_async: FXProp::new(false, None),
            attributes: Vec::new(),
            lifetimes: Vec::new(),
            generics: Vec::new(),
            where_bounds: Vec::new(),
            params: Vec::new(),
            body: Vec::new(),
            ret_stmt: None,
            ret_type: None,
            ret_mut: false,
            span: None,
        }
    }

    pub fn new_associated(name: syn::Ident) -> Self {
        Self {
            name,
            associated: true,
            vis: None,
            self_borrow: true,
            self_mut: false,
            self_ident: None,
            self_type: None,
            self_rc_ident: None,
            self_lifetime: None,
            is_async: FXProp::new(false, None),
            attributes: Vec::new(),
            lifetimes: Vec::new(),
            generics: Vec::new(),
            where_bounds: Vec::new(),
            params: Vec::new(),
            body: Vec::new(),
            ret_stmt: None,
            ret_type: None,
            ret_mut: false,
            span: None,
        }
    }

    pub fn add_lifetime(&mut self, lifetime: TokenStream) -> &mut Self {
        self.lifetimes.push(lifetime);
        self
    }

    #[allow(dead_code)]
    pub fn add_where_bound(&mut self, bound: TokenStream) -> &mut Self {
        self.where_bounds.push(bound);
        self
    }

    #[allow(dead_code)]
    pub fn add_param(&mut self, param: TokenStream) -> &mut Self {
        self.params.push(param);
        self
    }

    pub fn add_statement(&mut self, body: TokenStream) -> &mut Self {
        self.body.push(body);
        self
    }

    #[allow(dead_code)]
    pub fn maybe_add_generic(&mut self, generic: Option<TokenStream>) -> &mut Self {
        if let Some(generic) = generic {
            self.generics.push(generic);
        }
        self
    }

    fn _is_self_allowed(&self) -> darling::Result<()> {
        if self.associated {
            return Err(
                darling::Error::custom(format!("Associated function {} cannot have `self`", self.name))
                    .with_span(self.span.as_ref().unwrap_or(&self.name.span())),
            );
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_self_ident<T: ToTokens>(&mut self, ident: T) -> darling::Result<&mut Self> {
        self._is_self_allowed()?;
        self.self_ident = Some(ident.to_token_stream());
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn set_self_rc_ident<T: ToTokens>(&mut self, ident: T) -> darling::Result<&mut Self> {
        self._is_self_allowed()?;
        self.self_rc_ident = Some(ident.to_token_stream());
        Ok(self)
    }

    pub fn self_ident(&self) -> Option<TokenStream> {
        self.self_ident.clone().or_else(|| {
            if self.associated {
                None
            }
            else {
                let span = self.span();
                Some(quote_spanned! {span=> self})
            }
        })
    }

    pub fn span(&self) -> Span {
        #[allow(clippy::redundant_closure)]
        self.span.unwrap_or_else(|| Span::call_site())
    }

    pub fn self_maybe_rc(&mut self) -> Option<FXToksMeta> {
        self.self_rc_ident
            .as_ref()
            .map(|s_rc| FXToksMeta::new(s_rc.clone(), FXValueFlag::RefCounted))
            .or_else(|| self.self_ident().map(FXToksMeta::from))
    }

    pub fn self_maybe_rc_as_ref(&mut self) -> Option<FXToksMeta> {
        self.self_maybe_rc().map(|s| {
            if s.flags != FXValueFlag::RefCounted as u8 {
                s
            }
            else {
                let self_toks = s.to_token_stream();
                s.replace(quote_spanned! {self_toks.span()=> &#self_toks})
            }
        })
    }

    pub fn set_async(&mut self, is_async: FXProp<bool>) -> &mut Self {
        self.is_async = is_async;
        self
    }

    pub fn set_self_lifetime(&mut self, lifetime: TokenStream) -> &mut Self {
        self.add_lifetime(lifetime.clone());
        self.self_lifetime = Some(lifetime);
        self
    }

    pub fn self_lifetime(&self) -> &Option<TokenStream> {
        &self.self_lifetime
    }

    #[allow(dead_code)]
    pub fn is_async(&self) -> bool {
        *self.is_async
    }
}

impl FXConstructor for FXFnConstructor {
    fn fx_to_tokens(&self) -> TokenStream {
        let self_ident = self.self_ident();
        let name = &self.name;
        let span = self.span.unwrap_or(Span::call_site());
        let vis = &self.vis;
        let self_lifetime = &self.self_lifetime;
        let body = &self.body;
        let ret_stmt = &self.ret_stmt;
        let attributes = &self.attributes;

        let ret = if let Some(return_type) = &self.ret_type {
            quote_spanned! {span=> -> #return_type }
        }
        else {
            quote![]
        };

        let mut params = vec![];

        if !self.associated {
            let self_mut = if self.self_mut {
                quote_spanned! {span=> mut }
            }
            else {
                quote! {}
            };

            let self_spec = if self.self_borrow {
                quote_spanned! {span=> & #self_lifetime #self_mut }
            }
            else {
                quote_spanned! {span=> #self_mut }
            };

            params.push(if let Some(self_type) = &self.self_type {
                if self.self_borrow {
                    quote_spanned! {span=> #self_ident: #self_spec #self_type}
                }
                else {
                    quote_spanned! {span=> #self_spec #self_ident: #self_type}
                }
            }
            else {
                quote_spanned! {span=> #self_spec #self_ident }
            });
        }

        if !self.params.is_empty() {
            params.extend(self.params.iter().cloned());
        }

        let mut generic_params = vec![];

        if !self.lifetimes.is_empty() {
            let lifetimes = &self.lifetimes;
            generic_params.push(quote_spanned![span=> #(#lifetimes),*]);
        }
        if !self.generics.is_empty() {
            let generics = &self.generics;
            generic_params.push(quote_spanned![span=> #(#generics),*]);
        }

        let generic_params = if generic_params.is_empty() {
            quote![]
        }
        else {
            quote_spanned![span=> <#(#generic_params),*>]
        };

        let where_clause = if self.where_bounds.is_empty() {
            quote![]
        }
        else {
            let where_bounds = &self.where_bounds;
            quote_spanned![span=> where #(#where_bounds),*]
        };

        let async_decl = if *self.is_async {
            quote_spanned! {self.is_async.final_span()=> async }
        }
        else {
            quote![]
        };

        quote_spanned! {span=>
            #( #attributes )*
            #[allow(unknown_lints)]
            #[allow(clippy::type_complexity)]
            #vis #async_decl fn #name #generic_params (#( #params ),*) #ret
            #where_clause
            {
                #(#body)*
                #ret_stmt
            }
        }
    }

    #[inline]
    fn set_span(&mut self, span: Span) -> &mut Self {
        self.span = Some(span);
        self
    }

    #[inline]
    fn add_attribute(&mut self, attribute: syn::Attribute) -> &mut Self {
        self.attributes.push(attribute);
        self
    }
}

impl ToTokens for FXFnConstructor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.fx_to_tokens());
    }
}
