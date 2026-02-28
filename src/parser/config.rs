//! Config block parser.
//!
//! Parses the `config:` block containing agent_name, agent_label, etc.

use crate::ast::{ConfigBlock, Spanned};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::primitives::{
    dedent, indent, newline, skip_block_noise, spanned_string, to_ast_span, ParserInput, Span,
};

/// Parse an identifier or keyword that can be used as a field name.
fn field_name<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    String,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    select! {
        Token::Ident(s) => s.to_string(),
        Token::Description => "description".to_string(),
        Token::Label => "label".to_string(),
        Token::Source => "source".to_string(),
        Token::Target => "target".to_string(),
        Token::Welcome => "welcome".to_string(),
        Token::Error => "error".to_string(),
        Token::Instructions => "instructions".to_string(),
        Token::Messages => "messages".to_string(),
    }
}

/// Parse a config block entry.
fn config_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    (String, Spanned<String>),
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    field_name()
        .then_ignore(just(Token::Colon))
        .then(spanned_string())
        .map(|(name, value)| (name, value))
}

/// Parse the config block.
pub fn config_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ConfigBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Config)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(indent())
        .ignore_then(
            config_entry()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|entries, e| {
            let mut config = ConfigBlock {
                agent_name: Spanned::new(String::new(), 0..0),
                agent_label: None,
                description: None,
                agent_type: None,
                default_agent_user: None,
            };

            for (name, value) in entries {
                match name.as_str() {
                    "agent_name" => config.agent_name = value,
                    "agent_label" => config.agent_label = Some(value),
                    "description" => config.description = Some(value),
                    "agent_type" => config.agent_type = Some(value),
                    "default_agent_user" => config.default_agent_user = Some(value),
                    _ => {}
                }
            }

            Spanned::new(config, to_ast_span(e.span()))
        })
}
