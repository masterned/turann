#![allow(dead_code)]

#[derive(Default)]
pub struct MissingFields(Option<Vec<&'static str>>);

impl MissingFields {
    fn add(&mut self, field: &'static str) -> &Self {
        self.0.get_or_insert_default().push(field);
        self
    }

    fn add_if_none<T>(&mut self, field_name: &'static str, field: &Option<T>) -> &mut Self {
        if field.is_none() {
            self.add(field_name);
        }

        self
    }
}

impl MissingFields {
    fn as_builder_error(self) -> Result<(), BuilderError> {
        let Some(missing_fields) = self.0 else {
            return Ok(());
        };

        Err(BuilderError::missing_fields(&missing_fields))
    }
}

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
    fn _can_add_missing_fields() {
        let mut missing_fields = MissingFields::default();

        assert!(missing_fields.0.is_none());

        missing_fields.add("first");

        assert!(
            missing_fields
                .0
                .is_some_and(|fields| fields.contains(&"first"))
        );
    }

    #[test]
    fn _should_add_if_missing() {
        let mut missing_fields = MissingFields::default();

        missing_fields.add_if_none("first", &Some("value"));

        assert!(missing_fields.0.is_none());

        missing_fields.add_if_none::<()>("first again", &None);

        assert!(
            missing_fields
                .0
                .is_some_and(|fields| fields.contains(&"first again"))
        )
    }

    #[test]
    fn _can_convert_missing_fields_to_error() {
        let mut missing_fields = MissingFields::default();

        missing_fields
            .add_if_none::<()>("first", &None)
            .add_if_none::<()>("second", &None);

        let result = missing_fields.as_builder_error();

        assert_eq!(
            result,
            Err(BuilderError::InvalidState {
                message: "missing required field(s): `first`, `second`".into()
            })
        )
    }

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
