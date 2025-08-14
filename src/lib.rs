use syn::{self, parse_macro_input};
use target_struct::TargetStruct;

mod builder_attribute;
mod builder_error;
mod target_field;
mod target_struct;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    match TargetStruct::try_from(ast) {
        Ok(succ) => proc_macro2::TokenStream::from(succ).into(),
        Err(fail) => fail.into_compile_error().into(),
    }
}
