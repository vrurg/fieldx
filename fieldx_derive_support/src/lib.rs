use darling::{ast, FromDeriveInput};
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, token::Comma, DeriveInput};

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs)]
struct FXHelperStruct {
    pub(crate) vis:      syn::Visibility,
    pub(crate) ident:    syn::Ident,
    pub(crate) data:     ast::Data<(), syn::Field>,
    pub(crate) attrs:    Vec<syn::Attribute>,
    pub(crate) generics: syn::Generics,
}

#[proc_macro_attribute]
pub fn fxhelper(_args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // let attr_args = match ast::NestedMeta::parse_meta_list(args.into()) {
    //     Ok(v) => v,
    //     Err(e) => {
    //         return darling::Error::from(e).write_errors().into();
    //     }
    // };

    // let args = match FXSArgs::from_list(&attr_args) {
    //     Ok(v) => v,
    //     Err(e) => return darling::Error::from(e).write_errors().into(),
    // };

    let incopy = input.clone();
    let input_ast = parse_macro_input!(incopy as DeriveInput);
    let fx = match FXHelperStruct::from_derive_input(&input_ast) {
        Ok(v) => v,
        Err(e) => return darling::Error::from(e).write_errors().into(),
    };

    let FXHelperStruct {
        vis,
        ident,
        data,
        attrs,
        generics,
    } = &fx;

    let ast::Data::Struct(fields) = data
    else {
        panic!("Expected struct data")
    };

    let fields = &fields.fields;
    let attributes_method = if fields
        .iter()
        .find(|f| (*f).ident.as_ref().map_or("".to_string(), |i| i.to_string()) == "attributes")
        .is_some()
    {
        quote![
            fn attributes(&self) -> Option<&crate::helper::FXAttributes> {
                self.attributes.as_ref()
            }
        ]
    }
    else {
        quote![]
    };

    let mut getters_derive = quote![ #[derive(Getters)] ];
    for attr in attrs {
        if attr.path().is_ident("derive") {
            let args = attr
                .parse_args_with(Punctuated::<syn::Path, Comma>::parse_terminated)
                .unwrap();
            if args.iter().any(|a| a.is_ident("Getters")) {
                getters_derive = quote![];
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let getset_vis = vis.to_token_stream().to_string();

    quote! [
        #( #attrs )*
        #getters_derive
        #vis struct #ident #generics #where_clause {
            #[getset(skip)]
            rename:        Option<String>,
            #[getset(get = #getset_vis)]
            off:           ::darling::util::Flag,
            #[getset(skip)]
            attributes_fn: Option<crate::helper::FXAttributes>,
            #[getset(skip)]
            public: Option<crate::helper::FXNestingAttr<crate::helper::FXPubMode>>,
            #[getset(skip)]
            private: Option<crate::helper::FXWithOrig<bool, ::syn::Meta>>,

            #( #fields ),*
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            crate::util::validate_exclusives!{"visibility" => public, private}
        }

        impl #impl_generics crate::helper::FXHelperTrait for #ident #ty_generics #where_clause {
            fn is_true(&self) -> bool {
                !self.off.is_present()
            }

            fn rename(&self) -> Option<&str> {
                self.rename.as_deref()
            }

            fn attributes_fn(&self) -> Option<&FXAttributes> {
                self.attributes_fn.as_ref()
            }

            #attributes_method
        }
    ]
    .into()
}
