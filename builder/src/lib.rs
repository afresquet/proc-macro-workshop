use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, Data, DeriveInput, Field, GenericArgument,
    Ident, LitStr, PathArguments,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder = format_ident!("{name}Builder");

    let bad_attrs = fields(&input.data)
        .filter_map(has_bad_attribute)
        .collect::<Vec<_>>();
    if !bad_attrs.is_empty() {
        return quote! {
            #(#bad_attrs)*
        }
        .into();
    }

    let builder_fields = fields(&input.data).map(builder_field);
    let initial_builder_fields = fields(&input.data).map(initial_builder_field);
    let builder_methods = fields(&input.data).filter_map(builder_method);
    let builder_methods_each = fields(&input.data).filter_map(builder_method_each);
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
            #(#builder_methods_each)*

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
    Vec,
}

impl std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::Option => "Option",
            Self::Vec => "Vec",
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

fn has_bad_attribute(field: &Field) -> Option<TokenStream> {
    get_builder_attr_value_detail(field)
        .err()
        .map(syn::Error::into_compile_error)
}

fn get_builder_attr_each(field: &Field) -> Option<LitStr> {
    get_builder_attr_value_detail(field).ok().flatten()
}

fn get_builder_attr_value_detail(field: &Field) -> syn::Result<Option<LitStr>> {
    if unwrap_t(Wrapper::Vec, field).is_none() {
        return Ok(None);
    }
    let Some(attr) = field.attrs.first() else {
        return Ok(None);
    };
    if !attr.path().is_ident("builder") {
        return Ok(None);
    }
    let mut value = None;
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("each") {
            let v = meta.value()?;
            value = Some(v.parse::<LitStr>()?);
            Ok(())
        } else {
            Err(meta.error("expected `builder(each = \"...\")`"))
        }
    })?;
    Ok(value)
}

fn builder_field(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    let ty = unwrap_t(Wrapper::Option, field).unwrap_or(&field.ty);
    if unwrap_t(Wrapper::Vec, field).is_some() {
        quote! { #name: #ty }
    } else {
        quote! { #name: Option<#ty> }
    }
}

fn initial_builder_field(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    if unwrap_t(Wrapper::Vec, field).is_some() {
        quote! { #name: Vec::new() }
    } else {
        quote! { #name: None }
    }
}

fn builder_method(field: &Field) -> Option<TokenStream> {
    let name = field.ident.as_ref()?;
    if let Ok(Some(lit)) = get_builder_attr_value_detail(field) {
        if *name == lit.value() {
            return None;
        }
    }
    let ty = unwrap_t(Wrapper::Option, field).unwrap_or(&field.ty);
    if unwrap_t(Wrapper::Vec, field).is_some() {
        Some(quote! {
            fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = #name;
                self
            }
        })
    } else {
        Some(quote! {
            fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        })
    }
}

fn builder_method_each(field: &Field) -> Option<TokenStream> {
    let name = field.ident.as_ref()?;
    let ty = unwrap_t(Wrapper::Vec, field)?;
    let lit = get_builder_attr_each(field)?;
    let item_name = Ident::new(&lit.value(), lit.span());
    Some(quote! {
        fn #item_name(&mut self, #item_name: #ty) -> &mut Self {
            self.#name.push(#item_name);
            self
        }
    })
}

fn build_attribute(field: &Field) -> TokenStream {
    let Some(name) = &field.ident else {
        unimplemented!();
    };
    if unwrap_t(Wrapper::Option, field).is_some() || unwrap_t(Wrapper::Vec, field).is_some() {
        return quote! {
            #name: self.#name.clone()
        };
    }
    let err_msg = LitStr::new(&format!("missing field '{}'", name), name.span());
    quote! {
        #name: self.#name.clone().ok_or(#err_msg)?
    }
}
