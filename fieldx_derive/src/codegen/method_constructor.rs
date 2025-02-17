use getset::{Getters, Setters};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};

#[derive(Default, Debug, Getters, Setters)]
#[getset(get = "pub(crate)", set = "pub(crate)")]
pub(crate) struct MethodConstructor {
    name:          TokenStream,
    vis:           TokenStream,
    #[getset(skip)]
    is_async:      bool,
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
    self_borrow:   bool,
    self_mut:      bool,
    #[getset(get_mut)]
    attributes:    Vec<TokenStream>,
    #[getset(get_mut)]
    lifetimes:     Vec<TokenStream>,
    #[getset(get_mut)]
    generics:      Vec<TokenStream>,
    #[getset(get_mut)]
    where_bounds:  Vec<TokenStream>,
    #[getset(get_mut)]
    params:        Vec<TokenStream>,
    #[getset(get_mut)]
    body:          Vec<TokenStream>,
    ret_stmt:      TokenStream,
    ret_type:      TokenStream,
    ret_mut:       bool,
    #[getset(skip)]
    span:          Option<Span>,
}

impl MethodConstructor {
    pub(crate) fn new<T: ToTokens>(name: T) -> Self {
        Self {
            name: name.to_token_stream(),
            self_borrow: true,
            ..Default::default()
        }
    }

    pub(crate) fn add_lifetime(&mut self, lifetime: TokenStream) {
        self.lifetimes.push(lifetime);
    }

    #[allow(dead_code)]
    pub(crate) fn add_where_bound(&mut self, bound: TokenStream) {
        self.where_bounds.push(bound);
    }

    #[allow(dead_code)]
    pub(crate) fn add_param(&mut self, param: TokenStream) {
        self.params.push(param);
    }

    pub(crate) fn add_statement(&mut self, body: TokenStream) {
        self.body.push(body);
    }

    pub(crate) fn add_attribute(&mut self, attribute: TokenStream) {
        self.attributes.push(attribute);
    }

    pub(crate) fn maybe_add_attribute(&mut self, attribute: Option<TokenStream>) {
        if let Some(attribute) = attribute {
            self.add_attribute(attribute);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn maybe_add_generic(&mut self, generic: Option<TokenStream>) {
        if let Some(generic) = generic {
            self.generics.push(generic);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_self_ident(&mut self, ident: TokenStream) {
        self.self_ident = Some(ident);
    }

    pub(crate) fn self_ident(&self) -> TokenStream {
        self.self_ident.clone().unwrap_or_else(|| {
            let span = self.span();
            quote_spanned! {span=> self}
        })
    }

    pub(crate) fn set_span(&mut self, span: Span) {
        self.span = Some(span);
    }

    pub(crate) fn span(&self) -> Span {
        self.span.unwrap_or_else(|| Span::call_site())
    }

    pub(crate) fn self_maybe_rc(&mut self) -> TokenStream {
        if let Some(rc_ident) = &self.self_rc_ident {
            rc_ident.clone()
        }
        else {
            self.self_ident()
        }
    }

    pub(crate) fn set_async(&mut self, is_async: bool) {
        self.is_async = is_async;
    }

    pub fn set_self_lifetime(&mut self, lifetime: TokenStream) {
        self.add_lifetime(lifetime.clone());
        self.self_lifetime = Some(lifetime);
    }

    pub fn self_lifetime(&self) -> &Option<TokenStream> {
        &self.self_lifetime
    }

    #[allow(dead_code)]
    pub(crate) fn is_async(&self) -> bool {
        self.is_async
    }

    pub(crate) fn into_method(self) -> TokenStream {
        let self_ident = self.self_ident();
        let name = self.name;
        let span = self.span.unwrap_or(Span::call_site());
        let vis = self.vis;
        let self_lifetime = self.self_lifetime;

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
        let body = self.body;
        let ret_stmt = self.ret_stmt;
        let attributes = &self.attributes;
        let mut ret = self.ret_type;

        if !ret.is_empty() {
            ret = quote_spanned! {span=> -> #ret };
        }

        let mut params = vec![if let Some(self_type) = self.self_type {
            if self.self_borrow {
                quote_spanned! {span=> #self_ident: #self_spec #self_type}
            }
            else {
                quote_spanned! {span=> #self_spec #self_ident: #self_type}
            }
        }
        else {
            quote_spanned! {span=> #self_spec #self_ident }
        }];

        if self.params.len() > 0 {
            params.extend(self.params);
        }

        let mut generic_params = vec![];

        if self.lifetimes.len() > 0 {
            let lifetimes = self.lifetimes;
            generic_params.push(quote![#(#lifetimes),*]);
        }
        if self.generics.len() > 0 {
            let generics = self.generics;
            generic_params.push(quote![#(#generics),*]);
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

        let async_decl = if self.is_async {
            quote_spanned! {span=> async }
        }
        else {
            quote![]
        };

        quote_spanned! {span=>
            #( #attributes )*
            #vis #async_decl fn #name #generic_params (#( #params ),*) #ret
            #where_clause
            {
                #(#body)*
                #ret_stmt
            }
        }
    }
}
