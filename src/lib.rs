use proc_macro::TokenStream;
use quote::quote;
use syn::{self, parse_macro_input};

fn has_builder_attribute(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().segments[0].ident == "builder")
}

fn each_method_each_ident(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    attrs.iter().find_map(|syn::Attribute { meta, .. }| {
        let syn::Meta::List(syn::MetaList { tokens, .. }) = meta else {
            return None;
        };

        let mut tokens = tokens.clone().into_iter();

        if tokens.next().is_some_and(|t| {
            let proc_macro2::TokenTree::Ident(ident) = t else {
                return false;
            };

            ident == "each"
        }) && tokens.next().is_some_and(|t| {
            let proc_macro2::TokenTree::Punct(punct) = t else {
                return false;
            };

            punct.as_char() == '='
        }) {
            let Some(proc_macro2::TokenTree::Literal(literal)) = tokens.next() else {
                return None;
            };

            let syn::Lit::Str(literal) = syn::Lit::new(literal) else {
                return None;
            };

            Some(syn::Ident::new(&literal.value(), literal.span()))
        } else {
            None
        }
    })
}

fn inner_type(outer_type: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(outer_type) = outer_type else {
        return None;
    };

    if outer_type.qself.is_some() {
        return None;
    }

    let outer_type = &outer_type.path;

    if outer_type.segments.is_empty() {
        return None;
    }

    let last_segment = outer_type.segments.last()?;

    let syn::PathArguments::AngleBracketed(generics) = &last_segment.arguments else {
        return None;
    };

    if generics.args.len() != 1 {
        return None;
    }

    let syn::GenericArgument::Type(inner_type) = &generics.args[0] else {
        return None;
    };

    Some(inner_type)
}

fn each_method(
    syn::Field {
        attrs, ident, ty, ..
    }: &syn::Field,
) -> Option<(syn::Ident, proc_macro2::TokenStream)> {
    let each_ident = each_method_each_ident(attrs)?;
    let internal_ty = inner_type(ty)?.clone();
    let outer_ident = ident.clone()?;

    Some((
        each_ident.clone(),
        quote! {pub fn #each_ident(&mut self, #each_ident: impl Into<#internal_ty>) -> &mut Self {
            self.#outer_ident.get_or_insert_default().push(#each_ident.into());

            self
        }},
    ))
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let ident = &ast.ident;
    let builder_ident = syn::Ident::new(&format!("{ident}Builder"), ident.span());

    let each_methods = if let syn::Data::Struct(syn::DataStruct { ref fields, .. }) = ast.data {
        if let syn::Fields::Named(syn::FieldsNamed { named, .. }) = fields {
            named
                .iter()
                .filter(|&field| has_builder_attribute(field))
                .filter_map(|field| each_method(field))
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    let (each_methods_idents, each_methods): (Vec<_>, Vec<_>) = each_methods.into_iter().unzip();

    let fields = if let syn::Data::Struct(syn::DataStruct {
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

    let builder_methods = fields
        .iter()
        .filter(|&field| {
            !field
                .ident
                .clone()
                .is_some_and(|ident| each_methods_idents.contains(&ident))
        })
        .map(|field| {
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
            if p.path.segments.len() == 1 {
                if p.path.segments[0].ident == "Option" {
                    return quote! {
                        #ident: self.#ident.clone()
                    };
                } else if p.path.segments[0].ident == "Vec" {
                    return quote! {
                        #ident: self.#ident.clone().unwrap_or_default()
                    };
                }
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

            #(#each_methods)*

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
