use proc_macro2::{Span, TokenStream};
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn sorted(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast = input.clone();
    let ast = parse_macro_input!(ast as syn::Item);

    if let Err(error) = check_errors(ast) {
        let input: TokenStream = input.into();
        return quote::quote! {
            #error
            #input
        }
        .into();
    }

    input
}

fn check_errors(ast: syn::Item) -> Result<(), TokenStream> {
    let syn::Item::Enum(item_enum) = ast else {
        let error = syn::Error::new(Span::call_site(), "expected enum or match expression")
            .to_compile_error();
        return Err(error);
    };
    check_sorted(&item_enum)?;
    Ok(())
}

fn check_sorted(item_enum: &syn::ItemEnum) -> Result<(), TokenStream> {
    let variants = item_enum
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .collect::<Vec<_>>();
    let mut sorted_variants = variants.clone();
    sorted_variants.sort();

    for (actual, expected) in variants.into_iter().zip(sorted_variants.into_iter()) {
        if actual != expected {
            return Err(syn::Error::new(
                expected.span(),
                format!("{expected} should sort before {actual}"),
            )
            .to_compile_error());
        }
    }

    Ok(())
}
