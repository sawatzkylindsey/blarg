#[derive(Debug)]
pub struct DeriveParameter {
    pub name: syn::Ident,
}

pub fn build_parameters(fields: &syn::FieldsNamed) -> Vec<DeriveParameter> {
    fields
        .named
        .iter()
        .map(|field| DeriveParameter {
            name: field.ident.clone().unwrap(),
        })
        .collect()
}
