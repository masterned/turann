use std::error::Error;

use derive_builder::Builder;

#[derive(Debug, Builder)]
pub struct Target {
    #[builder(default = Self::default_id)]
    pub id: usize,
    #[builder(validate = Self::required_not_empty)]
    pub required: String,
    pub optional_unset: Option<String>,
    pub optional_set: Option<String>,
    #[builder(each = "empty")]
    pub vec_empty: Vec<String>,
    #[builder(each = "multi")]
    pub vec_multi: Vec<String>,
}

impl TargetBuilder {
    fn default_id() -> usize {
        1
    }

    fn required_not_empty(value: String) -> Result<String, TargetBuilderError> {
        if value.is_empty() {
            return Err(TargetBuilderError::InvalidField {
                field_name: "required".into(),
                message: "cannot be empty".into(),
            });
        }
        Ok(value)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut builder = Target::builder();

    let fail_build = builder.clone().build().unwrap_err();

    println!("{fail_build:#?}");

    builder
        .required("required")?
        .optional_set("optional")
        .multi("one")
        .multi("two".to_string());

    println!("{builder:#?}");

    let target = builder.build()?;

    println!("{target:#?}");

    let mut builder = Target::builder();

    let fail_set = builder.required("");

    println!("{fail_set:#?}");

    Ok(())
}
