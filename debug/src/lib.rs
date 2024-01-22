use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, LitStr};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let lit_name = LitStr::new(&name.to_string(), name.span());

    let fields_debug = fields(&input.data).map(field_debug);

    quote! {
        impl std::fmt::Debug for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(#lit_name)
                    #(#fields_debug)*
                    .finish()
            }
        }
    }
    .into()
}

fn fields(data: &Data) -> impl Iterator<Item = &Field> {
    match data {
        Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.named.iter(),
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn field_debug(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    let lit_name = LitStr::new(&name.to_string(), name.span());

    quote! { .field(#lit_name, &self.name) }
}
