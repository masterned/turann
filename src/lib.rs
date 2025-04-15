extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DataStruct, DeriveInput, parse_macro_input};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let builder_ident = syn::Ident::new(&format!("{ident}Builder"), ident.span());

    let fields = if let syn::Data::Struct(DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        unimplemented!()
    };

    let builder_fields = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;

        if let syn::Type::Path(p) = ty {
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "Option" {
                return quote! { #ident: #ty };
            }
        }

        quote! { #ident: std::option::Option<#ty> }
    });

    let builder_methods = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;

        if let syn::Type::Path(p) = ty {
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "Option" {
                return quote! { pub fn #ident(&mut self, #ident: impl Into<#ty>) -> &mut Self {
                    self.#ident = #ident.into();

                    self
                }};
            }
        }

        quote! { pub fn #ident(&mut self, #ident: impl Into<#ty>) -> &mut Self {
            let _ = self.#ident.insert(#ident.into());

            self
        }}
    });

    let result_fields = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;

        if let syn::Type::Path(p) = ty {
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "Option" {
                return quote! {
                    #ident: self.#ident.clone()
                };
            }
        }

        quote! {
            #ident: self.#ident.clone().ok_or(concat!(stringify!(#ident), " is not set"))?
        }
    });

    quote! {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct #builder_ident {
            #(#builder_fields,)*
        }

        impl #builder_ident {
            #(#builder_methods)*

            pub fn build(&self) -> std::result::Result<#ident, Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#result_fields,)*
                })
            }
        }

        impl #ident {
            pub fn builder () -> #builder_ident {
                #builder_ident::default()
            }
        }

    }
    .into()
}
