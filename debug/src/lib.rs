use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Field, GenericParam, Generics, Ident, LitStr,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let lit_name = LitStr::new(&name.to_string(), name.span());

    let fields_debug = fields(&input.data).map(field_debug);

    let bound = match input
        .attrs
        .iter()
        .map(outer_debug_attr)
        .collect::<syn::Result<Vec<_>>>()
    {
        Ok(predicates) => predicates.into_iter().fold(None, |a, b| a.or(b)),
        Err(err) => {
            let err = err.to_compile_error();
            return quote! {#err}.into();
        }
    };

    let generics = add_trait_bounds(input.generics, &input.data, bound.as_ref());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics std::fmt::Debug for #name #ty_generics #where_clause {
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

struct DebugBound {
    generic: Option<syn::Ident>,
    predicate: syn::WherePredicate,
}

fn outer_debug_attr(attr: &syn::Attribute) -> syn::Result<Option<DebugBound>> {
    if !attr.path().is_ident("debug") {
        return Ok(None);
    }
    let mut bound = None;
    attr.parse_nested_meta(|meta| {
        if !meta.path.is_ident("bound") {
            return Ok(());
        }
        let value = meta.value()?;
        let lit = value.parse::<LitStr>()?;
        let predicate = syn::parse_str::<syn::WherePredicate>(&lit.value())?;
        let generic = match &predicate {
            syn::WherePredicate::Type(p_ty) => match &p_ty.bounded_ty {
                syn::Type::Path(syn::TypePath { path, .. }) => {
                    if path.segments.len() > 1 {
                        let segment = path.segments.first().unwrap();
                        Some(segment.ident.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        };
        bound = Some(DebugBound { generic, predicate });
        Ok(())
    })?;
    Ok(bound)
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
        None => quote! { .field(#lit_name, &self.#name) },
    }
}

fn add_trait_bounds(mut generics: Generics, data: &Data, bound: Option<&DebugBound>) -> Generics {
    generics.make_where_clause();
    if let Some(where_clause) = generics.where_clause.as_mut() {
        if let Some(DebugBound { predicate, .. }) = bound {
            where_clause.predicates.push(predicate.clone());
        }
        for param in &mut generics.params {
            if let GenericParam::Type(type_param) = param {
                let phantom_data = fields(data).any(is_phantom_data_ty(&type_param.ident));
                let associated_types = fields(data)
                    .filter_map(get_associated_ty(&type_param.ident))
                    .collect::<Vec<_>>();
                let bound_attr = bound
                    .is_some_and(|db| db.generic.as_ref().is_some_and(|g| *g == type_param.ident));
                if !phantom_data && associated_types.is_empty() && !bound_attr {
                    type_param.bounds.push(parse_quote!(std::fmt::Debug));
                } else {
                    associated_types.iter().for_each(|ty| {
                        where_clause
                            .predicates
                            .push(parse_quote!(#ty: std::fmt::Debug));
                    });
                }
            }
        }
    }
    generics
}

fn is_phantom_data_ty<'a>(generic_ty: &'a Ident) -> impl Fn(&'a Field) -> bool {
    move |field| {
        if let Some(syn::Type::Path(ty)) = unwrap_t(Wrapper::PhantomData, field) {
            return ty.path.get_ident().is_some_and(|ty| ty == generic_ty);
        }
        false
    }
}

fn get_associated_ty<'a>(generic_ty: &'a Ident) -> impl Fn(&'a Field) -> Option<&'a syn::Type> {
    move |field| {
        let syn::Type::Path(ty) = &field.ty else {
            return None;
        };
        let segment = ty.path.segments.first()?;
        let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args, ..
        }) = &segment.arguments
        else {
            return None;
        };
        let syn::GenericArgument::Type(ty) = args.first()? else {
            return None;
        };
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            return None;
        };
        let segment = path.segments.first()?;
        if segment.ident == *generic_ty && path.segments.len() > 1 {
            Some(ty)
        } else {
            None
        }
    }
}

enum Wrapper {
    PhantomData,
}

impl std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::PhantomData => "PhantomData",
        };
        write!(f, "{string}")
    }
}

fn unwrap_t(wrapper: Wrapper, field: &Field) -> Option<&syn::Type> {
    let syn::Type::Path(ty) = &field.ty else {
        return None;
    };
    let segment = ty.path.segments.last()?;
    if segment.ident != wrapper.to_string() {
        return None;
    };
    let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return None;
    };
    if let Some(syn::GenericArgument::Type(ty)) = args.first() {
        Some(ty)
    } else {
        None
    }
}
