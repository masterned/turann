extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    eprintln!("{ast:#?}");

    let ident = &ast.ident;
    let builder_ident = syn::Ident::new(&format!("{ident}Builder"), ident.span());

    quote! {
        struct #builder_ident {

        }

        impl #ident {
            fn builder () -> #builder_ident {
                #builder_ident {}
            }
        }
    }
    .into()
}
