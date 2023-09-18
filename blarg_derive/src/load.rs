mod attribute;
mod parameter;
mod parser;

pub(self) fn incompatible_error(
    field_name: &syn::Ident,
    left: impl Into<String>,
    right: impl Into<String>,
) -> syn::Error {
    syn::Error::new(
        field_name.span(),
        format!(
            "Invalid - field cannot be both `{}` and `{}`.",
            left.into(),
            right.into(),
        ),
    )
}
