use crate::target_field::TargetField;
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
    pub fields: Vec<TargetField>,
}

impl TargetStruct {
    fn builder_fields(&self) -> proc_macro2::TokenStream {
        let builder_fields = self.fields.iter().map(TargetField::quote_builder_field);

        quote! { #(#builder_fields)* }
    }

    fn field_setters(&self) -> proc_macro2::TokenStream {
        let setters = self.fields.iter().map(TargetField::quote_setter);

        quote! { #(#setters)* }
    }

    fn result_fields(&self) -> proc_macro2::TokenStream {
        let result_fields = self
            .fields
            .iter()
            .map(|f| TargetField::quote_result_field(f));

        quote! { #(#result_fields)* }
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
        let field_setters = value.field_setters();
        let result_fields = value.result_fields();
        let field_attr_errors = value.field_attr_errors();
        let missing_fields_validators = value
            .fields
            .iter()
            .map(TargetField::quote_missing_validator);

        quote! {
            #field_attr_errors

            #[derive(Clone, Debug, Default)]
            pub struct #builder_ident {
                #builder_fields
            }

            impl #builder_ident {
                #field_setters

                pub fn build(&self) -> std::result::Result<#struct_ident, #builder_error_ident> {
                    let mut missing_fields = #missing_fields_ident::default();

                    #(#missing_fields_validators)*

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
