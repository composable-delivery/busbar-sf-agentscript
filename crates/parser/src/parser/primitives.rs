//! Primitive parsers for basic tokens.
//!
//! This module contains parsers for identifiers, strings, numbers,
//! newlines, indentation, and noise-skipping utilities.

use crate::ast::Spanned;
use crate::lexer::Token;
use chumsky::input::MappedInput;
use chumsky::prelude::*;

/// Token span type (from lexer).
pub type Span = SimpleSpan<usize>;

/// Spanned token type.
pub type SpannedToken<'src> = (Token<'src>, Span);

/// Parser input type - a slice of spanned tokens mapped into chumsky format.
/// Created by calling `tokens.split_token_span(eoi_span)` on a token slice.
pub type ParserInput<'tokens, 'src> =
    MappedInput<'tokens, Token<'src>, Span, &'tokens [SpannedToken<'src>]>;

/// Convert a chumsky SimpleSpan to our AST Span (Range<usize>).
pub fn to_ast_span(span: Span) -> std::ops::Range<usize> {
    span.start..span.end
}

/// Parse an identifier token.
pub fn ident<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    &'src str,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    select! {
        Token::Ident(s) => s,
    }
}

/// Parse an identifier as a spanned string.
pub fn spanned_ident<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<String>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    ident().map_with(|s, e| Spanned::new(s.to_string(), to_ast_span(e.span())))
}

/// Parse a string literal.
pub fn string_lit<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    &'src str,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    select! {
        Token::StringLit(s) => s,
    }
}

/// Parse a spanned string literal.
pub fn spanned_string<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<String>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    string_lit().map_with(|s, e| Spanned::new(s.to_string(), to_ast_span(e.span())))
}

/// Parse a number literal.
pub fn number_lit<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    f64,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    select! {
        Token::NumberLit(n) => n,
    }
}

/// Parse a newline token (for line tracking).
pub fn newline<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    just(Token::Newline).ignored()
}

/// Skip optional newlines.
#[allow(dead_code)]
pub fn skip_newlines<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    newline().repeated().ignored()
}

/// Parse an INDENT token.
pub fn indent<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    just(Token::Indent).ignored()
}

/// Parse a DEDENT token.
pub fn dedent<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    just(Token::Dedent).ignored()
}

/// Skip noise tokens (newlines, indents, dedents, comments) between blocks.
#[allow(dead_code)]
pub fn skip_noise<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    choice((newline(), indent(), dedent(), select! { Token::Comment(_) => () }))
        .repeated()
        .ignored()
}

/// Skip block noise (newlines and comments only - not indent/dedent).
pub fn skip_block_noise<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    choice((newline(), select! { Token::Comment(_) => () }))
        .repeated()
        .ignored()
}

/// Skip noise between top-level blocks (newlines, comments, AND dedents).
/// DEDENTs appear between blocks when exiting nested indented blocks.
pub fn skip_toplevel_noise<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    choice((newline(), dedent(), select! { Token::Comment(_) => () }))
        .repeated()
        .ignored()
}

/// Skip comments only (not newlines).
#[allow(dead_code)]
pub fn skip_comments<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    select! { Token::Comment(_) => () }.repeated().ignored()
}

/// Parse a description entry: `description: "..."`
pub fn description_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<String>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Description)
        .ignore_then(just(Token::Colon))
        .ignore_then(spanned_string())
}
