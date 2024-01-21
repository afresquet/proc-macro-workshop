use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Field};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder = format_ident!("{name}Builder");

    let builder_fields = fields(&input.data).map(builder_field);
    let initial_builder_fields = fields(&input.data).map(initial_builder_field);
    let builder_methods = fields(&input.data).map(builder_method);

    quote! {
        impl #name {
            pub fn builder() -> #builder {
                #builder {
                    #(#initial_builder_fields,)*
                }
            }
        }

        pub struct #builder {
            #(#builder_fields),*
        }

        impl #builder {
            #(#builder_methods)*
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

fn builder_field(field: &Field) -> TokenStream {
    let ty = &field.ty;
    if let Some(name) = &field.ident {
        return quote! { #name: Option<#ty> };
    }
    unimplemented!();
}

fn initial_builder_field(field: &Field) -> TokenStream {
    if let Some(name) = &field.ident {
        return quote! { #name: None };
    }
    unimplemented!();
}

fn builder_method(field: &Field) -> TokenStream {
    let ty = &field.ty;
    if let Some(name) = &field.ident {
        return quote! {
            fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        };
    }
    unimplemented!();
}
