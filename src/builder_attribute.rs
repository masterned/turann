use syn::parse_quote;

#[derive(Debug)]
pub enum BuilderAttribute {
    Each(syn::Ident),
    Validate(syn::Path),
    Default(syn::Path),
}

impl BuilderAttribute {
    fn get_validator_path(&self) -> std::option::Option<&syn::Path> {
        if let BuilderAttribute::Validate(path) = self {
            return Some(path);
        };

        None
    }
}

#[derive(Debug, Default)]
pub struct BuilderAttributes(pub std::vec::Vec<syn::Result<BuilderAttribute>>);

impl BuilderAttributes {
    pub fn iter(&self) -> std::slice::Iter<'_, syn::Result<BuilderAttribute>> {
        self.0.iter()
    }

    pub fn get_each_ident(&self) -> Option<&syn::Ident> {
        for attr in self {
            if let Ok(BuilderAttribute::Each(ident)) = attr {
                return Some(ident);
            }
        }

        None
    }

    pub fn get_first_validator_path(&self) -> std::option::Option<&syn::Path> {
        for attr in self {
            if let Ok(BuilderAttribute::Validate(path)) = attr {
                return Some(path);
            }
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
        if let Some(BuilderAttribute::Default(path)) = self.into_iter().flatten().next() {
            return Some(path);
        }

        None
    }
}

impl From<syn::Attribute> for BuilderAttributes {
    fn from(value: syn::Attribute) -> Self {
        let mut builder_attributes = vec![];

        if value.path().is_ident("builder") {
            if let Err(err) = value.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    let value = meta.value()?;
                    let litstr: syn::LitStr = value.parse()?;
                    let ident: syn::Ident = syn::parse_str(&litstr.value())?;

                    builder_attributes.push(Ok(BuilderAttribute::Each(ident)));

                    return Ok(());
                }

                if meta.path.is_ident("validate") {
                    let value = meta.value()?;
                    let path: syn::Path = value.parse()?;

                    builder_attributes.push(Ok(BuilderAttribute::Validate(path)));

                    return Ok(());
                }

                if meta.path.is_ident("default") {
                    builder_attributes.push(meta.value().map_or_else(
                        |_| {
                            Ok(BuilderAttribute::Default(parse_quote!(
                                std::default::Default::default
                            )))
                        },
                        |value| {
                            let path: syn::Path = value.parse()?;

                            Ok(BuilderAttribute::Default(path))
                        },
                    ));

                    return Ok(());
                }

                Err(meta.error(format!("builder attribute not recognized")))
            }) {
                builder_attributes.push(Err(err));
            };
        }

        BuilderAttributes(builder_attributes)
    }
}

impl IntoIterator for BuilderAttributes {
    type Item = syn::Result<BuilderAttribute>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BuilderAttributes {
    type Item = &'a syn::Result<BuilderAttribute>;

    type IntoIter = std::slice::Iter<'a, syn::Result<BuilderAttribute>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<syn::Result<BuilderAttribute>> for BuilderAttributes {
    fn from_iter<T: IntoIterator<Item = syn::Result<BuilderAttribute>>>(iter: T) -> Self {
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
