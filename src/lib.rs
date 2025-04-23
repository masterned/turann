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

        let Some(proc_macro2::TokenTree::Ident(ident)) = tokens.next() else {
            return None;
        };
        if ident != "each" {
            return None;
        }

        let Some(proc_macro2::TokenTree::Punct(punct)) = tokens.next() else {
            return None;
        };
        if punct.as_char() != '=' {
            return None;
        }

        let Some(proc_macro2::TokenTree::Literal(literal)) = tokens.next() else {
            return None;
        };

        let syn::Lit::Str(literal) = syn::Lit::new(literal) else {
            return None;
        };

        Some(syn::Ident::new(&literal.value(), literal.span()))
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

struct EachMethod {
    ident: syn::Ident,
    token_stream: proc_macro2::TokenStream,
}

impl EachMethod {
    fn ident_token_stream_tuple(self) -> (syn::Ident, proc_macro2::TokenStream) {
        (self.ident, self.token_stream)
    }
}

impl TryFrom<syn::Field> for EachMethod {
    type Error = ();

    fn try_from(
        syn::Field {
            ty, attrs, ident, ..
        }: syn::Field,
    ) -> Result<Self, Self::Error> {
        let each_ident = each_method_each_ident(&attrs).ok_or(())?;
        let internal_ty = inner_type(&ty).ok_or(())?.clone();
        let outer_ident = ident.clone().ok_or(())?;

        Ok(EachMethod {
            ident: each_ident.clone(),
            token_stream: quote! {pub fn #each_ident(&mut self, #each_ident: impl Into<#internal_ty>) -> &mut Self {
                self.#outer_ident.get_or_insert_default().push(#each_ident.into());

                self
            }},
        })
    }
}

fn builder_field(field: &syn::Field) -> proc_macro2::TokenStream {
    let ident = &field.ident;
    let ty = &field.ty;

    if let syn::Type::Path(p) = ty {
        if p.path.segments.len() == 1 && p.path.segments[0].ident == "Option" {
            return quote! { #ident: #ty };
        }
    }

    quote! { #ident: std::option::Option<#ty> }
}

fn is_not_each_method(each_methods_idents: &[syn::Ident], field: &syn::Field) -> bool {
    !field
        .ident
        .clone()
        .is_some_and(|ident| each_methods_idents.contains(&ident))
}

fn builder_method(field: &syn::Field) -> proc_macro2::TokenStream {
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
}

fn result_field(field: &syn::Field) -> proc_macro2::TokenStream {
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
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let ident = &ast.ident;
    let builder_ident = syn::Ident::new(&format!("{ident}Builder"), ident.span());

    let (each_methods_idents, each_methods) = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
            .iter()
            .filter(|&field| has_builder_attribute(field))
            .filter_map(|f| EachMethod::try_from(f.clone()).ok())
            .map(EachMethod::ident_token_stream_tuple)
            .unzip()
    } else {
        (vec![], vec![])
    };

    let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed {
            named: ref fields, ..
        }),
        ..
    }) = ast.data
    else {
        unimplemented!();
    };

    let builder_fields = fields.iter().map(builder_field);

    let builder_methods = fields
        .iter()
        .filter(|&field| is_not_each_method(&each_methods_idents, field))
        .map(builder_method);

    let result_fields = fields.iter().map(result_field);

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
