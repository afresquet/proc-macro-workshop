use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, Data, DeriveInput, Field, GenericArgument,
    LitStr, PathArguments,
};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder = format_ident!("{name}Builder");

    let builder_fields = fields(&input.data).map(builder_field);
    let initial_builder_fields = fields(&input.data).map(initial_builder_field);
    let builder_methods = fields(&input.data).map(builder_method);
    let build_attributes = fields(&input.data).map(build_attribute);

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

            pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
                Ok(#name {
                    #(#build_attributes,)*
                })
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

enum Wrapper {
    Option,
}

impl std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Wrapper::Option => "Option",
        };
        write!(f, "{string}")
    }
}

fn unwrap_t(wrapper: Wrapper, field: &Field) -> Option<&syn::Type> {
    let syn::Type::Path(ty) = &field.ty else {
        return None;
    };
    let Some(segment) = ty.path.segments.first() else {
        return None;
    };
    if segment.ident != wrapper.to_string() {
        return None;
    };
    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return None;
    };
    if let Some(GenericArgument::Type(ty)) = args.first() {
        Some(ty)
    } else {
        None
    }
}

fn builder_field(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    let ty = unwrap_t(Wrapper::Option, field).unwrap_or(&field.ty);
    quote! { #name: Option<#ty> }
}

fn initial_builder_field(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    quote! { #name: None }
}

fn builder_method(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    let ty = unwrap_t(Wrapper::Option, field).unwrap_or(&field.ty);
    quote! {
        fn #name(&mut self, #name: #ty) -> &mut Self {
            self.#name = Some(#name);
            self
        }
    }
}

fn build_attribute(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    if unwrap_t(Wrapper::Option, field).is_some() {
        return quote! {
            #name: self.#name.clone()
        };
    }
    let err_msg = LitStr::new(&format!("missing field '{}'", name), name.span());
    quote! {
        #name: self.#name.clone().ok_or(#err_msg)?
    }
}
