use std::error::Error;

use derive_builder::Builder;

#[derive(Debug, Builder)]
pub struct Command {
    pub executable: String,
    pub args: Vec<String>,
    pub current_dir: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let command = Command::builder()
        .executable("cargo")
        .args(["build".to_owned(), "--release".to_owned()])
        .current_dir("cwd".to_string())
        .build()?;

    println!("{command:#?}");

    Ok(())
}
