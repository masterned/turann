use syn::parse_quote;

#[derive(Debug)]
pub enum BuilderFieldAttribute {
    Each(syn::Ident),
    Validate(syn::Path),
    Default(syn::Path),
}

impl BuilderFieldAttribute {
    fn get_validator_path(&self) -> std::option::Option<&syn::Path> {
        if let BuilderFieldAttribute::Validate(path) = self {
            return Some(path);
        }

        None
    }
}

#[derive(Debug, Default)]
pub struct BuilderFieldAttributes(pub std::vec::Vec<syn::Result<BuilderFieldAttribute>>);

impl BuilderFieldAttributes {
    pub fn iter(&self) -> std::slice::Iter<'_, syn::Result<BuilderFieldAttribute>> {
        self.0.iter()
    }

    pub fn get_each_ident(&self) -> Option<&syn::Ident> {
        if let Some(BuilderFieldAttribute::Each(ident)) = self.into_iter().flatten().next() {
            return Some(ident);
        }

        None
    }

    pub fn get_first_validator_path(&self) -> std::option::Option<&syn::Path> {
        if let Some(BuilderFieldAttribute::Validate(path)) = self.into_iter().flatten().next() {
            return Some(path);
        }

        None
    }

    pub fn get_validator_paths(&self) -> std::vec::Vec<&syn::Path> {
        self.iter()
            .filter_map(|attr| {
                if let Ok(attr) = attr {
                    attr.get_validator_path()
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_default_path(&self) -> std::option::Option<&syn::Path> {
        if let Some(BuilderFieldAttribute::Default(path)) = self.into_iter().flatten().next() {
            return Some(path);
        }

        None
    }
}

impl From<syn::Attribute> for BuilderFieldAttributes {
    fn from(value: syn::Attribute) -> Self {
        let mut builder_attributes = vec![];

        if value.path().is_ident("builder") {
            if let Err(err) = value.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    let value = meta.value()?;
                    let litstr: syn::LitStr = value.parse()?;
                    let ident: syn::Ident = syn::parse_str(&litstr.value())?;

                    builder_attributes.push(Ok(BuilderFieldAttribute::Each(ident)));

                    return Ok(());
                }

                if meta.path.is_ident("validate") {
                    let value = meta.value()?;
                    let path: syn::Path = value.parse()?;

                    builder_attributes.push(Ok(BuilderFieldAttribute::Validate(path)));

                    return Ok(());
                }

                if meta.path.is_ident("default") {
                    builder_attributes.push(meta.value().map_or_else(
                        |_| {
                            Ok(BuilderFieldAttribute::Default(parse_quote!(
                                std::default::Default::default
                            )))
                        },
                        |value| {
                            let path: syn::Path = value.parse()?;

                            Ok(BuilderFieldAttribute::Default(path))
                        },
                    ));

                    return Ok(());
                }

                Err(meta.error("builder attribute not recognized".to_string()))
            }) {
                builder_attributes.push(Err(err));
            }
        }

        BuilderFieldAttributes(builder_attributes)
    }
}

impl IntoIterator for BuilderFieldAttributes {
    type Item = syn::Result<BuilderFieldAttribute>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BuilderFieldAttributes {
    type Item = &'a syn::Result<BuilderFieldAttribute>;

    type IntoIter = std::slice::Iter<'a, syn::Result<BuilderFieldAttribute>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<syn::Result<BuilderFieldAttribute>> for BuilderFieldAttributes {
    fn from_iter<T: IntoIterator<Item = syn::Result<BuilderFieldAttribute>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    mod builder_attributes {
        use super::*;

        #[test]
        #[ignore = "not yet implemented"]
        fn _prevent_multiple_each_attributes() {
            todo!()
        }

        #[test]
        #[ignore = "not yet implemented"]
        fn _require_field_with_each_attribute_to_be_container_type() {
            todo!()
        }
    }
}
