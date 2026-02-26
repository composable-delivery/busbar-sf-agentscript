//! Connection block parser.
//!
//! Parses `connection <name>:` blocks for escalation routing.
//!
//! # Syntax
//!
//! ```text
//! connection messaging:
//!    escalation_message: "I'm connecting you with a specialist."
//!    outbound_route_type: "OmniChannelFlow"
//!    outbound_route_name: "SpecialistQueue"
//! ```

use crate::ast::{ConnectionBlock, ConnectionEntry, Spanned};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::primitives::{
    dedent, indent, newline, skip_block_noise, spanned_ident, spanned_string, to_ast_span,
    ParserInput, Span,
};

/// Parse a connection entry (key: value pair).
fn connection_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ConnectionEntry>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    spanned_ident()
        .then_ignore(just(Token::Colon))
        .then(spanned_string())
        .map_with(|(name, value), e| {
            Spanned::new(ConnectionEntry { name, value }, to_ast_span(e.span()))
        })
}

/// Parse a connection block: `connection <name>:`
pub(crate) fn connection_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ConnectionBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Connection)
        .ignore_then(spanned_ident())
        .then_ignore(just(Token::Colon))
        .then_ignore(newline())
        .then_ignore(skip_block_noise())
        .then_ignore(indent())
        .then(
            connection_entry()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|(name, entries), e| {
            Spanned::new(ConnectionBlock { name, entries }, to_ast_span(e.span()))
        })
}

/// Parse the legacy `connections:` block and emit an error.
/// This parser exists to provide a helpful error message when users
/// use the old syntax.
pub(crate) fn legacy_connections_block<'tokens, 'src: 'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, (), extra::Err<Rich<'tokens, Token<'src>, Span>>>
       + Clone {
    just(Token::Connections)
        .then_ignore(just(Token::Colon))
        .validate(|_, e, emitter| {
            emitter.emit(Rich::custom(
                e.span(),
                "Invalid syntax: 'connections:' is not valid. Use 'connection <name>:' instead.\n\
                 Example:\n\
                 connection messaging:\n\
                    escalation_message: \"I'm connecting you...\"\n\
                    outbound_route_type: \"OmniChannelFlow\"\n\
                    outbound_route_name: \"SpecialistQueue\"",
            ));
        })
}
