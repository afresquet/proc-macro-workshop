use itertools::{Itertools, MultiPeek};
use proc_macro2::{Group, Ident, Literal, TokenStream, TokenTree};
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
        let _ = input.parse::<syn::Token![in]>()?; // Discard
        let start = input.parse::<syn::LitInt>()?.base10_parse()?;
        let _ = input.parse::<syn::Token![..]>()?; // Discard
        let end = input.parse::<syn::LitInt>()?.base10_parse()?;
        let content;
        let _ = syn::braced!(content in input); // Discard
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
    let mut iter = content.into_iter().multipeek();
    let mut output = Vec::new();

    while let Some(tt) = iter.next() {
        let tt = match tt {
            TokenTree::Ident(ref ident) => {
                parse_ident(&mut iter, ident, substitution, target).unwrap_or(tt)
            }
            TokenTree::Group(group) => {
                let stream = substitute_target(group.stream(), target, substitution);
                Group::new(group.delimiter(), stream).into()
            }
            tt => tt,
        };
        output.push(tt);
    }

    TokenStream::from_iter(output)
}

fn parse_ident(
    iter: &mut MultiPeek<proc_macro2::token_stream::IntoIter>,
    ident: &Ident,
    substitution: &Literal,
    target: &Ident,
) -> Option<TokenTree> {
    let is_tilde = iter
        .peek()
        .is_some_and(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == '~'));
    let is_target = iter
        .peek()
        .is_some_and(|tt| matches!(tt, TokenTree::Ident(i) if i == target));
    let is_prefix = is_tilde && is_target;

    let is_tilde = iter
        .peek()
        .is_some_and(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == '~'));
    let is_ident = iter
        .peek()
        .is_some_and(|tt| matches!(tt, TokenTree::Ident(_)));
    let is_suffix = is_tilde && is_ident;

    match (ident, is_prefix, is_suffix) {
        (prefix, true, true) => {
            iter.next(); // ~
            iter.next(); // N
            iter.next(); // ~
            let Some(TokenTree::Ident(suffix)) = iter.next() else {
                unreachable!();
            };
            let name = format!("{prefix}{}{suffix}", substitution.clone());
            Some(syn::Ident::new(&name, ident.span()).into())
        }
        (prefix, true, _) => {
            iter.next(); // ~
            iter.next(); // N
            let name = format!("{prefix}{}", substitution.clone());
            Some(syn::Ident::new(&name, ident.span()).into())
        }
        (ident, _, _) if ident == target => Some(substitution.clone().into()),
        (_, _, _) => None,
    }
}
