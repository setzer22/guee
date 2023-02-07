use syn::parse_macro_input;

mod derive_builder;

mod color_macro;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn guee_derive_builder(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    derive_builder::guee_derive_builder_2(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn color(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    color_macro::color_macro2(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
