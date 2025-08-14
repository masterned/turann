#[derive(Debug)]
pub enum BuilderAttribute {
    Each(syn::Ident),
    Validate(syn::Path),
}

#[derive(Debug, Default)]
pub struct BuilderAttributes(pub std::vec::Vec<syn::Result<BuilderAttribute>>);

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

impl FromIterator<syn::Result<BuilderAttribute>> for BuilderAttributes {
    fn from_iter<T: IntoIterator<Item = syn::Result<BuilderAttribute>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
