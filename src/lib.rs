#![allow(dead_code)]

use quote::quote;
use syn::{self, parse_macro_input, parse_quote, spanned::Spanned};

mod builder_error;

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

fn extract_fields_named(input: &syn::DeriveInput) -> syn::Result<&syn::FieldsNamed> {
    match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields_named) => Ok(fields_named),
            syn::Fields::Unnamed(_) => Err(syn::Error::new(
                input.ident.span(),
                "Cannot create Builder for tuple structs",
            )),
            syn::Fields::Unit => Err(syn::Error::new(
                input.ident.span(),
                "Cannot create Builder for unit structs",
            )),
        },
        syn::Data::Enum(_) => Err(syn::Error::new(
            input.span(),
            "Cannot create Builder for enums",
        )),
        syn::Data::Union(_) => Err(syn::Error::new(
            input.ident.span(),
            "Cannot create Builder for unions",
        )),
    }
}

#[derive(Debug)]
enum BuilderAttribute {
    Each(syn::Ident),
    Validate(syn::Path),
}

#[derive(Debug, Default)]
struct BuilderAttributes(pub std::vec::Vec<syn::Result<BuilderAttribute>>);

impl From<syn::Attribute> for BuilderAttributes {
    fn from(value: syn::Attribute) -> Self {
        let mut builder_attributes = vec![];

        if value.path().is_ident("builder") {
            if let Err(err) = value.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    let value = meta.value()?;
                    let litstr: syn::LitStr = value.parse()?;
                    let ident: syn::Ident = syn::parse_str(&litstr.value())?;

                    builder_attributes.push(Ok(BuilderAttribute::Each(ident)));

                    return Ok(());
                }

                if meta.path.is_ident("validate") {
                    let value = meta.value()?;
                    let path: syn::Path = value.parse()?;

                    builder_attributes.push(Ok(BuilderAttribute::Validate(path)));

                    return Ok(());
                }

                Err(meta.error(format!("builder attribute not recognized")))
            }) {
                builder_attributes.push(Err(err));
            };
        }

        BuilderAttributes(builder_attributes)
    }
}

impl IntoIterator for BuilderAttributes {
    type Item = syn::Result<BuilderAttribute>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<syn::Result<BuilderAttribute>> for BuilderAttributes {
    fn from_iter<T: IntoIterator<Item = syn::Result<BuilderAttribute>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[derive(Debug)]
struct TargetField {
    pub ident: syn::Ident,
    pub ty: syn::Type,
    pub builder_attributes: BuilderAttributes,
}

impl TargetField {
    fn quote_validated_setter(&self, builder_error_ident: syn::Ident) -> proc_macro2::TokenStream {
        let field_ident = &self.ident;
        let field_type = &self.ty;

        let BuilderAttribute::Validate(validator) = self.builder_attributes.0[0].as_ref().unwrap()
        else {
            return quote! { /*the validator messed up*/ };
        };

        quote! {
            pub fn #field_ident(&mut self, #field_ident: impl Into<#field_type>) -> std::result::Result<&mut Self, #builder_error_ident> {
                #validator(#field_ident).map_err(|msg| #builder_error_ident::InvalidField {field_name: #field_ident.into(), message: msg.into()})?;

                let _ = self.#field_ident.insert(#field_ident.into());

                Ok(self)
            }
        }
    }

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

    fn is_option_field(&self) -> bool {
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

    fn get_each_ident(&self) -> std::option::Option<syn::Ident> {
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

#[derive(Debug)]
struct TargetStruct {
    pub ident: syn::Ident,
    pub fields: Vec<TargetField>,
}

impl TargetStruct {
    fn builder_fields(&self) -> proc_macro2::TokenStream {
        let builder_fields = self.fields.iter().map(TargetField::quote_builder_field);

        quote! { #(#builder_fields,)* }
    }

    fn field_setters(&self) -> proc_macro2::TokenStream {
        let setters = self
            .fields
            .iter()
            .filter(|f| f.get_each_ident().is_none())
            .map(TargetField::quote_setter);

        quote! { #(#setters)* }
    }

    fn field_each_methods(&self) -> proc_macro2::TokenStream {
        let each_methods = self
            .fields
            .iter()
            .filter_map(TargetField::quote_each_method);

        quote! { #(#each_methods)* }
    }

    fn result_fields(&self) -> proc_macro2::TokenStream {
        let struct_ident = &self.ident;
        let builder_error_ident =
            syn::Ident::new(&format!("{struct_ident}BuilderError"), struct_ident.span());
        let uninitialized_error_path: syn::Path =
            parse_quote! {#builder_error_ident::missing_field};
        let result_fields = self
            .fields
            .iter()
            .map(|f| TargetField::quote_result_field(f, uninitialized_error_path.clone()));

        quote! { #(#result_fields,)* }
    }

    fn field_attr_errors(&self) -> proc_macro2::TokenStream {
        let field_attr_errors = self.fields.iter().map(TargetField::quote_attr_errors);

        quote! { #(#field_attr_errors)* }
    }
}

impl TryFrom<syn::DeriveInput> for TargetStruct {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let fields_named = extract_fields_named(&input)?;

        let struct_ident = &input.ident;

        let fields = fields_named
            .named
            .iter()
            .cloned()
            .filter_map(|f| f.try_into().ok())
            .collect();

        Ok(Self {
            ident: struct_ident.clone(),
            fields,
        })
    }
}

impl From<TargetStruct> for proc_macro2::TokenStream {
    fn from(value: TargetStruct) -> Self {
        let struct_ident = &value.ident;
        let struct_ident_string = struct_ident.to_string();
        let builder_ident = syn::Ident::new(&format!("{struct_ident}Builder"), struct_ident.span());
        let builder_error_ident =
            syn::Ident::new(&format!("{struct_ident}BuilderError"), struct_ident.span());
        let missing_fields_ident = syn::Ident::new(
            &format!("Missing{struct_ident}Fields",),
            struct_ident.span(),
        );
        let builder_fields = value.builder_fields();
        let builder_methods = value.field_setters();
        let each_methods = value.field_each_methods();
        let result_fields = value.result_fields();
        let field_attr_errors = value.field_attr_errors();
        let missing_fields_checks = value
            .fields
            .iter()
            .filter(|field| !field.is_option_field())
            .filter(|field| {
                if let syn::Type::Path(ref p) = field.ty {
                    p.path.segments.len() != 1 || p.path.segments[0].ident != "Vec"
                } else {
                    false
                }
            })
            .map(|field| {
                let field_ident = &field.ident;
                let field_ident_string = field_ident.to_string();
                quote! { missing_fields.add_if_none(#field_ident_string, &self.#field_ident); }
            });

        quote! {
            #field_attr_errors

            #[derive(Clone, Debug, Default, PartialEq)]
            pub struct #builder_ident {
                #builder_fields
            }

            impl #builder_ident {
                #builder_methods

                #each_methods

                pub fn build(&self) -> std::result::Result<#struct_ident, #builder_error_ident> {
                    let mut missing_fields = #missing_fields_ident::default();

                    #(#missing_fields_checks)*

                    missing_fields.as_builder_error()?;

                    Ok(#struct_ident {
                        #result_fields
                    })
                }
            }

            #[derive(Default)]
            pub struct #missing_fields_ident(std::option::Option<Vec<&'static str>>);

            impl #missing_fields_ident {
                fn add(&mut self, field: &'static str) -> &Self {
                    self.0.get_or_insert_default().push(field);
                    self
                }

                fn add_if_none<T>(&mut self, field_name: &'static str, field: &std::option::Option<T>) -> &mut Self {
                    if field.is_none() {
                        self.add(field_name);
                    }

                    self
                }

                fn as_builder_error(self) -> std::result::Result<(), #builder_error_ident> {
                    let Some(missing_fields) = self.0 else {
                        return Ok(());
                    };

                    Err(#builder_error_ident::missing_fields(&missing_fields))
                }
            }

            /// Occurs when the user either tries to incorrectly assign a field,
            /// or when they attempt to build the target struct while the builder
            /// is in an invalid state.
            #[derive(Clone, Debug, PartialEq)]
            pub enum #builder_error_ident {
                /// Typically occurs on the `build()` method. Examples include:
                /// missing fields, constraint violations, and illogical structs.
                InvalidState {
                    message: std::borrow::Cow<'static, str>,
                },
                /// Typically occurs on the setter functions. Allows the builder
                /// to catch problems before the user attempts to build the target.
                InvalidField {
                    field_name: std::borrow::Cow<'static, str>,
                    message: std::borrow::Cow<'static, str>,
                },
            }

            impl #builder_error_ident {
                pub fn missing_fields(fields: &[&str]) -> Self {
                    let missing_field_names = fields
                        .iter()
                        .map(|field_name| format!("`{field_name}`"))
                        .reduce(|acc, next| format!("{acc}, {next}"))
                        .unwrap_or_default();
                    Self::InvalidState {
                        message: format!("missing required field(s): {missing_field_names}").into(),
                    }
                }

                pub fn missing_field(field: &str) -> Self {
                    Self::missing_fields(&[field])
                }
            }

            impl std::fmt::Display for #builder_error_ident {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #builder_error_ident::InvalidState { message } => {
                            write!(f, "Unable to build {}: {}", #struct_ident_string, message)
                        }
                        #builder_error_ident::InvalidField {
                            field_name,
                            message,
                        } => write!(f, "Unable to assign field `{field_name}`: {message}"),
                    }
                }
            }

            impl std::error::Error for #builder_error_ident {}

            impl #struct_ident {
                pub fn builder() -> #builder_ident {
                    #builder_ident::default()
                }
            }

        }
    }
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    match TargetStruct::try_from(ast) {
        Ok(succ) => proc_macro2::TokenStream::from(succ).into(),
        Err(fail) => fail.into_compile_error().into(),
    }
}
