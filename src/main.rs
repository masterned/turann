use std::error::Error;

use derive_builder::Builder;

#[derive(Debug, Builder)]
pub struct Target {
    pub required: String,
    pub optional_unset: Option<String>,
    pub optional_set: Option<String>,
    #[builder(each = "empty")]
    pub vec_empty: Vec<String>,
    #[builder(each = "multi")]
    pub vec_multi: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut builder = Target::builder();

    let fail_build = builder.clone().build().unwrap_err();

    println!("{fail_build:#?}");

    builder
        .required("required")
        .optional_set("optional")
        .multi("one")
        .multi("two".to_string());

    println!("{builder:#?}");

    let target = builder.build()?;

    println!("{target:#?}");

    Ok(())
}
