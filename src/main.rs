#![allow(dead_code)]

use std::error::Error;

use derive_builder::Builder;

#[derive(Debug, Builder)]
pub struct Command {
    pub executable: String,
    #[builder(validate = CommandBuilder::validate_user)]
    pub user: String,
    #[builder(each = "arg")]
    pub args: Vec<String>,
    #[builder(each = "env")]
    pub env: Vec<String>,
    pub current_dir: Option<String>,
}

impl CommandBuilder {
    fn validate_user(user: String) -> Result<(), &'static str> {
        if user.is_empty() {
            return Err("cannot be empty string");
        }

        Ok(())
    }
}

fn as_b_e(validation_result: Result<(), &'static str>) -> Result<(), CommandBuilderError> {
    validation_result.map_err(|result| CommandBuilderError::InvalidField {
        field_name: "field_name".into(),
        message: result.into(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let command = Command::builder()
        .arg("build")
        .arg("--release")
        .current_dir("cwd".to_string())
        .build();

    println!("{command:#?}");

    Ok(())
}
