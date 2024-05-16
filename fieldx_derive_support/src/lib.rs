use std::collections::HashMap;

use darling::{ast, FromDeriveInput, FromField};
use proc_macro;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, punctuated::Punctuated, token::Comma, DeriveInput};

#[derive(Debug, FromField, Clone)]
#[darling(attributes(fxhelper), forward_attrs)]
struct FXHelperField {
    ident: Option<syn::Ident>,
    vis:   syn::Visibility,
    ty:    syn::Type,
    attrs: Vec<syn::Attribute>,

    exclusive: Option<String>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs)]
struct FXHelperStruct {
    vis:      syn::Visibility,
    ident:    syn::Ident,
    data:     ast::Data<(), FXHelperField>,
    attrs:    Vec<syn::Attribute>,
    generics: syn::Generics,
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

    let mut fields_tt: Vec<TokenStream> = Vec::new();
    let mut exclusives: HashMap<String, Vec<(syn::Ident, TokenStream)>> = HashMap::new();
    let mut exclusives_tt: Vec<TokenStream> = vec![];

    exclusives.insert(
        "visibility".to_string(),
        vec![
            (format_ident!("public"), quote![is_some]),
            (format_ident!("private"), quote![is_some]),
        ],
    );

    for field in fields.iter() {
        if let Some(exclusive) = &field.exclusive {
            if let Some(ref ident) = field.ident {
                let ident = ident.clone();
                let check_method = if let syn::Type::Path(ref tpath) = field.ty {
                    if tpath.path.is_ident("Flag") {
                        quote![is_present]
                    }
                    else {
                        quote![is_some]
                    }
                }
                else {
                    return darling::Error::unexpected_type(&field.ty.to_token_stream().to_string())
                        .write_errors()
                        .into();
                };

                if exclusives.contains_key(exclusive) {
                    exclusives.get_mut(exclusive).unwrap().push((ident, check_method));
                }
                else {
                    exclusives.insert(exclusive.clone(), vec![(ident, check_method)]);
                }
            }
        }

        let FXHelperField {
            ident, vis, ty, attrs, ..
        } = &field;

        fields_tt.push(quote![ #( #attrs )* #vis #ident: #ty ])
    }

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
        quote![
            fn attributes(&self) -> Option<&crate::helper::FXAttributes> {
                None
            }
        ]
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

    for (group, fields) in exclusives.iter() {
        let mut checks: Vec<TokenStream> = vec![];

        for (ident, check_method) in fields.iter() {
            let ident_str = ident.to_string();
            checks.push(quote![
                if self.#ident.#check_method() {
                    set_params.push(#ident_str);
                }
            ]);
        }

        exclusives_tt.push(quote![
            {
                let mut set_params: Vec<&str> = vec![];
                #(#checks)*

                if set_params.len() > 1 {
                    let err = darling::Error::custom(
                        format!(
                            "The following options from group '{}' cannot be used together: {}",
                            #group,
                            set_params.iter().map(|f| format!("`{}`", f)).collect::<Vec<String>>().join(", ") ));
                    return Err(err);
                }
            }
        ]);
    }

    quote! [
        #( #attrs )*
        #[derive(FromMeta, Clone)]
        #[darling(and_then = Self::__validate_helper)]
        #getters_derive
        #vis struct #ident #generics #where_clause {
            #[getset(skip)]
            rename:        Option<String>,
            #[getset(get = #getset_vis)]
            off:           Flag,
            #[getset(skip)]
            attributes_fn: Option<crate::helper::FXAttributes>,
            #[getset(skip)]
            public: Option<crate::helper::FXNestingAttr<crate::helper::FXPubMode>>,
            #[getset(skip)]
            private: Option<crate::helper::FXWithOrig<bool, ::syn::Meta>>,

            #( #fields_tt ),*
        }

        impl #impl_generics crate::helper::FXTriggerHelper for #ident #ty_generics #where_clause {
            fn is_true(&self) -> bool {
                !self.off.is_present()
            }
        }

        impl #impl_generics crate::helper::FXHelperTrait for #ident #ty_generics #where_clause {
            fn rename(&self) -> Option<&str> {
                self.rename.as_deref()
            }

            fn attributes_fn(&self) -> Option<&crate::helper::FXAttributes> {
                self.attributes_fn.as_ref()
            }

            fn public_mode(&self) -> Option<crate::helper::FXPubMode> {
                if self.private.is_some() {
                    Some(crate::helper::FXPubMode::Private)
                }
                else {
                    self.public.as_ref().map(|pm| (**pm).clone())
                }
            }

            #attributes_method
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            fn validate_exclusives(&self) -> ::darling::Result<()> {
                #(#exclusives_tt)*
                Ok(())
            }

            fn __validate_helper(self) -> ::darling::Result<Self> {
                self.validate_exclusives()?;
                Ok(self)
            }
        }
    ]
    .into()
}
