use std::{borrow::Cow, error::Error};

use derive_builder::Builder;

#[derive(Debug, Builder)]
pub struct Command {
    pub executable: String,
    #[builder(each = "arg")]
    pub args: Vec<String>,
    #[builder(each = "env")]
    pub env: Vec<String>,
    pub current_dir: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let command = Command::builder()
        .executable("cargo")
        .arg("build")
        .arg("--release")
        .current_dir("cwd".to_string())
        .build()?;

    println!("{command:#?}");

    Ok(())
}
