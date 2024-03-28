use crate::{
    fields::FXField,
    helper::FXHelper,
    input_receiver::FXInputReceiver,
    util::args::{self, FXSArgs},
};
use delegate::delegate;
use getset::{CopyGetters, Getters};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::cell::{OnceCell, RefCell};
use syn;

#[derive(Getters, CopyGetters)]
pub(crate) struct FXCodeGenCtx {
    errors: RefCell<OnceCell<darling::error::Accumulator>>,

    #[getset(get = "pub")]
    args: FXSArgs,

    #[getset(get = "pub")]
    input: FXInputReceiver,

    needs_builder_struct: RefCell<Option<bool>>,

    tokens: RefCell<OnceCell<TokenStream>>,
}

pub(crate) struct FXFieldCtx<'f> {
    field:      &'f FXField,
    pub_tok:    OnceCell<TokenStream>,
    ty_tok:     OnceCell<TokenStream>,
    ty_wrapped: OnceCell<TokenStream>,
    ident:      OnceCell<Option<&'f syn::Ident>>,
    ident_tok:  OnceCell<TokenStream>,
}

impl FXCodeGenCtx {
    delegate! {
        to self.args {
            pub fn is_sync(&self) -> bool;
            pub fn needs_new(&self) -> bool;
            pub fn needs_into(&self) -> Option<bool>;
        }
    }

    pub fn new(input: FXInputReceiver, args: args::FXSArgs) -> Self {
        Self {
            input,
            args,
            errors: RefCell::new(OnceCell::new()),
            tokens: RefCell::new(OnceCell::new()),
            needs_builder_struct: RefCell::new(None),
        }
    }

    pub fn push_error(&self, err: darling::Error) {
        let mut errors = self.errors.borrow_mut();
        errors.get_or_init(|| darling::Error::accumulator());
        errors.get_mut().unwrap().push(err);
    }

    pub fn input_ident(&self) -> &syn::Ident {
        &self.input.ident()
    }

    pub fn tokens_extend(&self, toks: TokenStream) {
        let mut tokens = self.tokens.borrow_mut();
        tokens.get_or_init(|| TokenStream::new());
        tokens.get_mut().unwrap().extend(toks);
    }

    pub fn finalize(&self) -> TokenStream {
        let mut errors = self.errors.borrow_mut();
        match errors.take() {
            Some(errs) => match errs.finish() {
                Ok(_) => (),
                Err(err) => {
                    self.tokens_extend(TokenStream::from(darling::Error::from(err).write_errors()));
                }
            },
            None => (),
        };

        self.tokens.borrow_mut().take().unwrap_or_else(|| TokenStream::new())
    }

    pub fn needs_builder_struct(&self) -> Option<bool> {
        (*self.needs_builder_struct.borrow()).or(self.args.needs_builder())
    }

    pub fn require_builder(&self) {
        let mut nb_ref = self.needs_builder_struct.borrow_mut();
        if nb_ref.is_none() {
            *nb_ref = Some(true);
        }
    }
}

impl<'f> FXFieldCtx<'f> {
    delegate! {
        to self.field {
            pub fn needs_accessor(&self, is_sync: bool) -> bool;
            pub fn needs_accessor_mut(&self) -> bool;
            pub fn needs_reader(&self) -> bool;
            pub fn needs_writer(&self) -> bool;
            pub fn needs_setter(&self) -> bool;
            pub fn needs_clearer(&self) -> bool;
            pub fn needs_predicate(&self) -> bool;
            pub fn needs_into(&self) -> Option<bool>;
            pub fn needs_builder(&self) -> Option<bool>;
            pub fn is_into(&self) -> bool;
            pub fn is_lazy(&self) -> bool;
            pub fn is_ignorable(&self) -> bool;
            pub fn is_optional(&self) -> bool;
            pub fn has_default(&self) -> bool;
            #[allow(dead_code)]
            pub fn is_pub(&self) -> bool;
            pub fn span(&self) -> &Span;
            pub fn vis(&self) -> &syn::Visibility;
            pub fn ty(&self) -> &syn::Type;
            pub fn attrs(&self) -> &Vec<syn::Attribute>;
            pub fn lazy(&self) -> &Option<FXHelper>;
            pub fn base_name(&self) -> &Option<String>;
            pub fn accessor(&self) -> &Option<FXHelper>;
            pub fn accessor_mut(&self) -> &Option<FXHelper>;
            pub fn builder(&self) -> &Option<FXHelper>;
            pub fn reader(&self) -> &Option<FXHelper>;
            pub fn writer(&self) -> &Option<FXHelper>;
            pub fn setter(&self) -> &Option<FXHelper>;
            pub fn clearer(&self) -> &Option<FXHelper>;
            pub fn predicate(&self) -> &Option<FXHelper>;
            #[allow(dead_code)]
            pub fn private(&self) -> &Option<bool>;
            pub fn default(&self) -> &Option<syn::Meta>;
        }
    }

    pub fn new(field: &'f FXField) -> Self {
        Self {
            field,
            pub_tok: OnceCell::new(),
            ty_wrapped: OnceCell::new(),
            ident: OnceCell::new(),
            ident_tok: OnceCell::new(),
            ty_tok: OnceCell::new(),
        }
    }

    #[inline]
    pub fn field(&self) -> &'f FXField {
        self.field
    }

    #[inline]
    pub fn pub_tok(&self) -> &TokenStream {
        self.pub_tok.get_or_init(|| {
            // eprintln!("+++ INIT pub_tok of {}: {} // from {:?}", self.ident_tok(), self.is_pub(), self.private());
            if self.field.is_pub() {
                quote![pub]
            }
            else {
                TokenStream::new()
            }
        })
    }

    pub fn ty_tok(&self) -> &TokenStream {
        self.ty_tok.get_or_init(|| self.field.ty().to_token_stream())
    }

    #[inline]
    pub fn ty_wrapped<F>(&self, initializer: F) -> &TokenStream
    where
        F: FnOnce() -> TokenStream,
    {
        self.ty_wrapped.get_or_init(initializer)
    }

    #[inline]
    pub fn ident(&self) -> Option<&'f syn::Ident> {
        self.ident.get_or_init(|| self.field.ident().as_ref()).clone()
    }

    #[inline]
    pub fn ident_tok(&self) -> &TokenStream {
        self.ident_tok.get_or_init(|| match self.ident() {
            Some(ident) => ident.to_token_stream(),
            None => TokenStream::new(),
        })
    }

    pub fn for_ident_str(&self) -> String {
        match self.ident() {
            Some(ident) => format!(" for field '{}'", ident.to_string()),
            None => String::new(),
        }
    }

    pub fn helper_base_name(&self) -> Option<String> {
        if let Some(base_name) = self.base_name() {
            Some(base_name.clone())
        }
        else if let Some(ident) = self.field.ident() {
            // let ident = self.field.ident();
            Some(ident.to_string())
        }
        else {
            None
        }
    }
}
