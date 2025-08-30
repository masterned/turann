use crate::{builder_attribute::BuilderStructAttributes, target_field::TargetField};
use quote::quote;
use syn::{self, spanned::Spanned};

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
pub struct TargetStruct {
    pub ident: syn::Ident,
    pub builder_ident: syn::Ident,
    pub builder_error_ident: syn::Ident,
    pub fields: Vec<TargetField>,
    pub missing_fields_ident: syn::Ident,
    pub attributes: BuilderStructAttributes,
}

impl TargetStruct {
    fn quote_builder_struct(&self) -> proc_macro2::TokenStream {
        let builder_ident = &self.builder_ident;
        let builder_fields = self.fields.iter().map(TargetField::quote_builder_field);

        quote! {
            #[derive(Clone, Debug, Default)]
            pub struct #builder_ident {
                #(#builder_fields)*
            }
        }
    }

    fn quote_builder_impl(&self) -> proc_macro2::TokenStream {
        let struct_ident = &self.ident;
        let builder_ident = &self.builder_ident;
        let builder_error_ident = &self.builder_error_ident;

        let field_setters = self
            .fields
            .iter()
            .map(|field| field.quote_setter(&builder_error_ident));

        let missing_fields_ident = &self.missing_fields_ident;

        let missing_fields_validators =
            self.fields.iter().map(TargetField::quote_missing_validator);

        let result_fields = self.fields.iter().map(TargetField::quote_result_field);

        let return_value = if let Some(validator_path) = self.attributes.get_validator_path() {
            quote! {
                #validator_path(result)
            }
        } else {
            quote! {
                Ok(result)
            }
        };

        quote! {
            impl #builder_ident {
                #(#field_setters)*

                pub fn build(&self) -> std::result::Result<#struct_ident, #builder_error_ident> {
                    let mut missing_fields = #missing_fields_ident::default();

                    #(#missing_fields_validators)*

                    missing_fields.as_builder_error()?;

                    let result = #struct_ident {
                        #(#result_fields)*
                    };

                    #return_value
                }
            }
        }
    }

    fn quote_missing_fields_block(&self) -> proc_macro2::TokenStream {
        let missing_fields_ident = &self.missing_fields_ident;
        let builder_error_ident = &self.builder_error_ident;

        quote! {
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
        }
    }

    fn quote_builder_error_block(&self) -> proc_macro2::TokenStream {
        let builder_error_ident = &self.builder_error_ident;
        let struct_ident_string = self.ident.to_string();

        quote! {
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
        }
    }

    fn quote_struct_impl(&self) -> proc_macro2::TokenStream {
        let struct_ident = &self.ident;
        let builder_ident = &self.builder_ident;

        quote! {
            impl #struct_ident {
                pub fn builder() -> #builder_ident {
                    #builder_ident::default()
                }
            }
        }
    }
}

impl TryFrom<syn::DeriveInput> for TargetStruct {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let fields_named = extract_fields_named(&input)?;

        let struct_ident = &input.ident;
        let builder_ident = syn::Ident::new(&format!("{struct_ident}Builder"), struct_ident.span());
        let builder_error_ident =
            syn::Ident::new(&format!("{}Error", &builder_ident), struct_ident.span());

        let fields = fields_named
            .named
            .iter()
            .cloned()
            .filter_map(|f| f.try_into().ok())
            .collect();

        let missing_fields_ident = syn::Ident::new(
            &format!("Missing{}Fields", struct_ident),
            struct_ident.span(),
        );

        let attributes = input
            .attrs
            .iter()
            .cloned()
            .flat_map(BuilderStructAttributes::from)
            .collect();

        Ok(Self {
            ident: struct_ident.clone(),
            builder_ident,
            builder_error_ident,
            fields,
            missing_fields_ident,
            attributes,
        })
    }
}

impl From<TargetStruct> for proc_macro2::TokenStream {
    fn from(value: TargetStruct) -> Self {
        let field_attr_errors = value.fields.iter().map(TargetField::quote_attr_errors);
        let builder_struct = value.quote_builder_struct();
        let builder_impl = value.quote_builder_impl();
        let missing_fields_block = value.quote_missing_fields_block();
        let builder_error_block = value.quote_builder_error_block();
        let struct_impl = value.quote_struct_impl();

        quote! {
            #(#field_attr_errors)*

            #builder_struct

            #builder_impl

            #missing_fields_block

            #builder_error_block

            #struct_impl
        }
    }
}
