use quote::quote;
use syn::{self, PathArguments, spanned::Spanned};

use crate::builder_attribute::BuilderAttributes;

fn is_container(ident: &'static str, ty: &syn::Type) -> bool {
    let syn::Type::Path(p) = ty else {
        return false;
    };

    if p.qself.is_some() {
        return false;
    }

    let Some(segment) = p.path.segments.last() else {
        return false;
    };

    segment.ident == ident && matches!(segment.arguments, PathArguments::AngleBracketed(_))
}

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
    fn is_optional(&self) -> bool {
        is_container("Option", &self.ty)
    }

    fn is_vec(&self) -> bool {
        is_container("Vec", &self.ty)
    }

    fn has_each_method(&self) -> bool {
        self.builder_attributes.get_each_ident().is_some()

        // FIXME: move `Vec` validation to `BuilderAttributes`
        && self.is_vec()
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

    pub fn quote_builder_field(&self) -> proc_macro2::TokenStream {
        let ident = &self.ident;
        let ty = &self.ty;

        if self.is_optional() || self.has_each_method() {
            return quote! { pub #ident: #ty, };
        }

        quote! { pub #ident: std::option::Option<#ty>, }
    }

    pub fn quote_setter(&self) -> proc_macro2::TokenStream {
        let field_ident = &self.ident;
        let field_type = &self.ty;

        if self.is_optional() {
            let inner_type = inner_type(&self.ty).unwrap().clone();

            return quote! {
                pub fn #field_ident(&mut self, value: impl Into<#inner_type>) -> &mut Self {
                    let value = value.into();

                    let _ = self.#field_ident.insert(value);

                    self
                }
            };
        }

        if let Some(each_ident) = self.builder_attributes.get_each_ident() {
            let inner_type = inner_type(&self.ty).unwrap().clone();
            return quote! {
                pub fn #each_ident(&mut self, value: impl Into<#inner_type>) -> &mut Self {
                    let value = value.into();

                    self.#field_ident.push(value);

                    self
                }
            };
        }

        quote! {
            pub fn #field_ident(&mut self, value: impl Into<#field_type>) -> &mut Self {
                let value = value.into();

                let _ = self.#field_ident.insert(value);

                self
            }
        }
    }

    pub fn quote_missing_validator(&self) -> proc_macro2::TokenStream {
        if self.is_optional() || self.is_vec() {
            return quote! {};
        }

        let field_ident = &self.ident;
        let field_ident_string = field_ident.to_string();

        quote! { missing_fields.add_if_none(#field_ident_string, &self.#field_ident); }
    }

    pub fn quote_result_field(&self) -> proc_macro2::TokenStream {
        let field_ident = &self.ident;

        if self.is_optional() || self.has_each_method() {
            return quote! {
                #field_ident: self.#field_ident.clone(),
            };
        }

        quote! {
            #field_ident: self.#field_ident.clone().unwrap(),
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

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
}
