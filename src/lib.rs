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

        quote! { #ident: std::option::Option<#ty> }
    });

    let builder_methods = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;

        quote! { pub fn #ident(&mut self, #ident: impl Into<#ty>) -> &mut Self {
            let _ = self.#ident.insert(#ident.into());

            self
        }}
    });

    let build_fields = fields.iter().map(|field| {
        let ident = &field.ident;

        quote! {
            #ident: self.#ident.clone().ok_or(concat!(stringify!(#ident), " is not set"))?
        }
    });

    quote! {
        #[derive(Clone, Debug, Default, PartialEq)]
        struct #builder_ident {
            #(#builder_fields,)*
        }

        impl #builder_ident {
            #(#builder_methods)*

            fn build(&self) -> std::result::Result<#ident, Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#build_fields,)*
                })
            }
        }

        impl #ident {
            fn builder () -> #builder_ident {
                #builder_ident::default()
            }
        }

    }
    .into()
}
