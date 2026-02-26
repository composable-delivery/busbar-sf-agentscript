//! Language block parser.
//!
//! Parses the `language:` block.

use crate::ast::{LanguageBlock, LanguageEntry, Spanned};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::expressions::expr;
use super::primitives::{
    dedent, indent, newline, skip_block_noise, spanned_ident, to_ast_span, ParserInput, Span,
};

/// Parse a language entry (key: value pair).
fn language_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<LanguageEntry>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    spanned_ident()
        .then_ignore(just(Token::Colon))
        .then(expr())
        .map_with(|(name, value), e| {
            Spanned::new(LanguageEntry { name, value }, to_ast_span(e.span()))
        })
}

/// Parse the language block.
pub(crate) fn language_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<LanguageBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Language)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            language_entry()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|entries, e| Spanned::new(LanguageBlock { entries }, to_ast_span(e.span())))
}
