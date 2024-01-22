use proc_macro2::{Span, TokenStream};
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn sorted(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::Item);

    let syn::Item::Enum(_) = input else {
        return syn::Error::new(Span::call_site(), "expected enum or match expression")
            .to_compile_error()
            .into();
    };

    TokenStream::new().into()
}
