use itertools::{Itertools, MultiPeek};
use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
use syn::parse_macro_input;

#[derive(Debug)]
struct Sequence {
    ident: syn::Ident,
    range: SequenceRange,
    content: TokenStream,
}

impl syn::parse::Parse for Sequence {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        let _ = input.parse::<syn::Token![in]>()?; // Discard
        let start = input.parse::<syn::LitInt>()?.base10_parse()?;
        let _ = input.parse::<syn::Token![..]>()?; // Discard
        let inclusive = input.peek(syn::Token![=]);
        if inclusive {
            let _ = input.parse::<syn::Token![=]>()?; // Discard
        }
        let end = input.parse::<syn::LitInt>()?.base10_parse()?;
        let range = SequenceRange::new(start, end, inclusive);
        let content;
        let _ = syn::braced!(content in input); // Discard
        let content = content.parse()?;
        Ok(Sequence {
            ident,
            range,
            content,
        })
    }
}

#[derive(Clone, Debug)]
enum SequenceRange {
    Exclusive(std::ops::Range<usize>),
    Inclusive(std::ops::RangeInclusive<usize>),
}

impl SequenceRange {
    pub fn new(start: usize, end: usize, inclusive: bool) -> Self {
        if inclusive {
            Self::Inclusive(start..=end)
        } else {
            Self::Exclusive(start..end)
        }
    }
}

impl Iterator for SequenceRange {
    type Item = Literal;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self {
            SequenceRange::Exclusive(range) => range.next(),
            SequenceRange::Inclusive(range) => range.next(),
        };
        next.map(Literal::usize_unsuffixed)
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Sequence {
        ident,
        range,
        content,
    } = parse_macro_input!(input as Sequence);

    if has_repeat_section(content.clone()) {
        repeat_section(content.clone(), &ident, range).into()
    } else {
        range
            .map(|substitution| substitute_target(content.clone(), &ident, &substitution))
            .collect::<TokenStream>()
            .into()
    }
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

fn repeat_section(
    content: TokenStream,
    target: &syn::Ident,
    literals: SequenceRange,
) -> TokenStream {
    content
        .into_iter()
        .map(|tt| match tt {
            TokenTree::Group(group) if is_repeat_section(group.clone()) => Group::new(
                group.delimiter(),
                literals
                    .clone()
                    .map(|substitution| {
                        let mut stream = group.stream().into_iter();
                        let Some(TokenTree::Group(content)) = stream.nth(1) else {
                            unreachable!()
                        };
                        substitute_target(content.stream(), target, &substitution)
                    })
                    .collect(),
            )
            .into(),
            _ => tt,
        })
        .collect()
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

fn has_repeat_section(content: TokenStream) -> bool {
    content.into_iter().any(|tt| {
        if let TokenTree::Group(group) = tt {
            is_repeat_section(group)
        } else {
            false
        }
    })
}

fn is_repeat_section(group: Group) -> bool {
    let mut iter = group.stream().into_iter();
    match (iter.next(), iter.next(), iter.next()) {
        (
            Some(TokenTree::Punct(left)),
            Some(TokenTree::Group(group)),
            Some(TokenTree::Punct(right)),
        ) => {
            left.as_char() == '#'
                && group.delimiter() == Delimiter::Parenthesis
                && right.as_char() == '*'
        }
        _ => false,
    }
}
