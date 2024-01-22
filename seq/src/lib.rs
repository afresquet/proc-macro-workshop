use proc_macro2::{Group, Literal, TokenStream, TokenTree};
use syn::parse_macro_input;

#[derive(Debug)]
struct Sequence {
    ident: syn::Ident,
    start: usize,
    end: usize,
    content: TokenStream,
}

impl syn::parse::Parse for Sequence {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        input.parse::<syn::Token![in]>()?;
        let start = input.parse::<syn::LitInt>()?.base10_parse()?;
        input.parse::<syn::Token![..]>()?;
        let end = input.parse::<syn::LitInt>()?.base10_parse()?;
        let content;
        syn::braced!(content in input);
        let content = content.parse()?;
        Ok(Sequence {
            ident,
            start,
            end,
            content,
        })
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Sequence {
        ident,
        start,
        end,
        content,
    } = parse_macro_input!(input as Sequence);
    eprintln!("{content:#?}");

    (start..end)
        .map(Literal::usize_unsuffixed)
        .map(|substitution| substitute_target(content.clone(), &ident, &substitution))
        .collect::<TokenStream>()
        .into()
}

fn substitute_target(
    content: TokenStream,
    target: &syn::Ident,
    substitution: &Literal,
) -> TokenStream {
    content
        .into_iter()
        .map(|tt| match tt {
            TokenTree::Ident(ref ident) if ident == target => substitution.clone().into(),
            TokenTree::Group(group) => {
                let stream = substitute_target(group.stream(), target, substitution);
                Group::new(group.delimiter(), stream).into()
            }
            tt => tt,
        })
        .collect()
}
