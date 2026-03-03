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

/// Parse a `connections:` wrapper block containing named connection sub-blocks.
///
/// # Syntax
///
/// ```text
/// connections:
///    messaging:
///       escalation_message: "I'm connecting you with a specialist."
///       outbound_route_type: "OmniChannelFlow"
///       outbound_route_name: "SpecialistQueue"
/// ```
///
/// Each named sub-block is equivalent to a `connection <name>:` block.
pub(crate) fn connections_wrapper_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Vec<Spanned<ConnectionBlock>>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    // Each named sub-block: `messaging:` with indented key-value entries
    let named_sub_block = spanned_ident()
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
        });

    just(Token::Connections)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            named_sub_block
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
}
