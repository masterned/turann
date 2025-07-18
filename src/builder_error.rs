#[allow(dead_code)]
/// Occurs when the user either tries to incorrectly assign a field,
/// or when they attempt to build the target struct while the builder
/// is in an invalid state.
#[derive(Clone, Debug, PartialEq)]
pub enum BuilderError {
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

#[allow(dead_code)]
impl BuilderError {
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

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuilderError::InvalidState { message } => {
                write!(f, "Unable to build #Target: {message}")
            }
            BuilderError::InvalidField {
                field_name,
                message,
            } => write!(f, "Unable to assign field `{field_name}`: {message}"),
        }
    }
}

impl std::error::Error for BuilderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn _missing_fields_formatted_correctly() {
        let result = BuilderError::missing_fields(&["one", "two", "three"]);

        assert_eq!(
            result,
            BuilderError::InvalidState {
                message: "missing required field(s): `one`, `two`, `three`".into()
            }
        )
    }
}
