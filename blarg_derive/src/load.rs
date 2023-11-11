mod attribute;
mod choices;
mod parameter;
mod parser;

pub(self) fn incompatible_error(
    context: &str,
    field_name: &syn::Ident,
    left: impl Into<String>,
    right: impl Into<String>,
) -> syn::Error {
    syn::Error::new(
        field_name.span(),
        format!(
            "Invalid - {context} cannot be both `{}` and `{}`.",
            left.into(),
            right.into(),
        ),
    )
}
