use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, LitStr};

#[proc_macro_derive(CustomDebug, attributes(debug))]
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

fn debug_attr(field: &Field) -> Option<LitStr> {
    let attr = field.attrs.first()?;
    if !attr.path().is_ident("debug") {
        return None;
    }
    let syn::Meta::NameValue(meta) = &attr.meta else {
        return None;
    };
    let syn::Expr::Lit(expr) = &meta.value else {
        return None;
    };
    if let syn::Lit::Str(lit_str) = &expr.lit {
        Some(lit_str.clone())
    } else {
        None
    }
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
    match debug_attr(field) {
        Some(debug) => quote! { .field(#lit_name, &format_args!(#debug, &self.#name)) },
        None => quote! { .field(#lit_name, &self.name) },
    }
}
