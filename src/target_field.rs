use quote::quote;
use syn::{self, spanned::Spanned};

use crate::builder_attribute::{BuilderAttribute, BuilderAttributes};

fn inner_type(outer_type: &syn::Type) -> std::option::Option<&syn::Type> {
    let syn::Type::Path(outer_type) = outer_type else {
        return std::option::Option::None;
    };

    if outer_type.qself.is_some() {
        return std::option::Option::None;
    }

    let outer_type = &outer_type.path;

    if outer_type.segments.is_empty() {
        return std::option::Option::None;
    }

    let last_segment = outer_type.segments.last()?;

    let syn::PathArguments::AngleBracketed(generics) = &last_segment.arguments else {
        return std::option::Option::None;
    };

    if generics.args.len() != 1 {
        return std::option::Option::None;
    }

    let syn::GenericArgument::Type(inner_type) = &generics.args[0] else {
        return std::option::Option::None;
    };

    std::option::Option::Some(inner_type)
}

#[derive(Debug)]
pub struct TargetField {
    pub ident: syn::Ident,
    pub ty: syn::Type,
    pub builder_attributes: BuilderAttributes,
}

impl TargetField {
    fn quote_basic_setter(&self) -> proc_macro2::TokenStream {
        let field_ident = &self.ident;
        let field_type = &self.ty;

        quote! {pub fn #field_ident(&mut self, #field_ident: impl Into<#field_type>) -> &mut Self {
            let _ = self.#field_ident.insert(#field_ident.into());

            self
        }}
    }

    fn quote_optional_setter(&self) -> proc_macro2::TokenStream {
        let field_ident = &self.ident;
        let field_type = &self.ty;

        quote! { pub fn #field_ident(&mut self, #field_ident: impl Into<#field_type>) -> &mut Self {
            self.#field_ident = #field_ident.into();

            self
        }}
    }

    pub fn is_option_field(&self) -> bool {
        if let syn::Type::Path(ref p) = self.ty {
            p.path.segments.len() == 1 && p.path.segments[0].ident == "Option"
        } else {
            false
        }
    }

    pub fn quote_setter(&self) -> proc_macro2::TokenStream {
        if self.is_option_field() {
            self.quote_optional_setter()
        } else {
            self.quote_basic_setter()
        }
    }

    pub fn get_each_ident(&self) -> std::option::Option<syn::Ident> {
        for attr in &self.builder_attributes.0 {
            if let Ok(BuilderAttribute::Each(ident)) = attr {
                return std::option::Option::Some(ident.clone());
            }
        }
        std::option::Option::None
    }

    pub fn quote_each_method(&self) -> std::option::Option<proc_macro2::TokenStream> {
        let each_ident = self.get_each_ident()?;
        let internal_ty = inner_type(&self.ty)?.clone();
        let outer_ident = &self.ident;

        std::option::Option::Some(
            quote! {pub fn #each_ident(&mut self, #each_ident: impl Into<#internal_ty>) -> &mut Self {
                self.#outer_ident.get_or_insert_default().push(#each_ident.into());

                self
            }},
        )
    }

    pub fn quote_builder_field(&self) -> proc_macro2::TokenStream {
        let ident = &self.ident;
        let ty = &self.ty;

        if let syn::Type::Path(p) = ty {
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "Option" {
                return quote! { #ident: #ty };
            }
        }

        quote! { #ident: std::option::Option<#ty> }
    }

    pub fn quote_result_field(
        &self,
        uninitialized_error_path: syn::Path,
    ) -> proc_macro2::TokenStream {
        let field_ident = &self.ident;
        let field_ident_string = field_ident.to_string();

        if let syn::Type::Path(p) = &self.ty {
            if p.path.segments.len() == 1 {
                match &p.path.segments[0].ident {
                    opt if opt == "Option" => {
                        return quote! {
                            #field_ident: self.#field_ident.clone()
                        };
                    }
                    vec if vec == "Vec" => {
                        return quote! {
                            #field_ident: self.#field_ident.clone().unwrap_or_default()
                        };
                    }
                    _ => (),
                }
            }
        }

        quote! {
            #field_ident: self.#field_ident.clone().ok_or(#uninitialized_error_path(#field_ident_string))?
        }
    }

    pub fn quote_attr_errors(&self) -> proc_macro2::TokenStream {
        let errors = self.builder_attributes.0.iter().filter_map(|a| match a {
            Ok(_) => std::option::Option::None,
            Err(e) => proc_macro2::TokenStream::from(e.to_compile_error()).into(),
        });

        quote! {
            #(#errors)*
        }
    }
}

impl TryFrom<syn::Field> for TargetField {
    type Error = syn::Error;

    fn try_from(
        ref field @ syn::Field {
            ref ident,
            ref attrs,
            ref ty,
            ..
        }: syn::Field,
    ) -> syn::Result<Self> {
        let builder_attributes = attrs
            .iter()
            .cloned()
            .flat_map(BuilderAttributes::from)
            .collect();

        Ok(Self {
            ident: ident
                .clone()
                .ok_or_else(|| syn::Error::new(field.span(), "Unable to find field ident"))?,
            ty: ty.clone(),
            builder_attributes,
        })
    }
}
