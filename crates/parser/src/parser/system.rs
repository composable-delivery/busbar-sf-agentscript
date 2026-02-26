//! System block parser.
//!
//! Parses the `system:` block containing messages and instructions.

use crate::ast::{Instructions, Spanned, SystemBlock, SystemMessages};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::instructions::simple_instructions;
use super::primitives::{
    dedent, indent, newline, skip_block_noise, spanned_string, to_ast_span, ParserInput, Span,
};

/// Parse a message entry (welcome: "..." or error: "...").
fn message_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    (&'static str, Spanned<String>),
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        just(Token::Welcome)
            .ignore_then(just(Token::Colon))
            .ignore_then(spanned_string())
            .map(|s| ("welcome", s)),
        just(Token::Error)
            .ignore_then(just(Token::Colon))
            .ignore_then(spanned_string())
            .map(|s| ("error", s)),
    ))
}

/// Parse system messages sub-block.
fn system_messages<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<SystemMessages>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Messages)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(indent())
        .ignore_then(
            message_entry()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|entries, e| {
            let mut msgs = SystemMessages {
                welcome: None,
                error: None,
            };
            for (name, value) in entries {
                match name {
                    "welcome" => msgs.welcome = Some(value),
                    "error" => msgs.error = Some(value),
                    _ => {}
                }
            }
            Spanned::new(msgs, to_ast_span(e.span()))
        })
}

/// Parse a system block entry.
fn system_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    SystemEntry,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        system_messages().map(SystemEntry::Messages),
        simple_instructions().map(SystemEntry::Instructions),
    ))
}

/// Parse the system block.
pub(crate) fn system_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<SystemBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::System)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            system_entry()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|entries, e| {
            let mut sys = SystemBlock {
                messages: None,
                instructions: None,
            };
            for entry in entries {
                match entry {
                    SystemEntry::Messages(m) => sys.messages = Some(m),
                    SystemEntry::Instructions(i) => sys.instructions = Some(i),
                }
            }
            Spanned::new(sys, to_ast_span(e.span()))
        })
}

/// Helper enum for system block parsing.
enum SystemEntry {
    Messages(Spanned<SystemMessages>),
    Instructions(Spanned<Instructions>),
}
