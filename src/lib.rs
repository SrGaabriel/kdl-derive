use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod expand;
mod model;

#[proc_macro_derive(ToKdl, attributes(kdl))]
pub fn derive_to_kdl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match model::Container::from_derive_input(&input) {
        Ok(container) => expand::expand_to_kdl(&container).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(FromKdl, attributes(kdl))]
pub fn derive_from_kdl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match model::Container::from_derive_input(&input) {
        Ok(container) => expand::expand_from_kdl(&container).into(),
        Err(err) => err.to_compile_error().into(),
    }
}
