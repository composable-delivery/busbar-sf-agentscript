//! Topic and StartAgent block parser.
//!
//! Parses `topic` and `start_agent` blocks.

use crate::ast::{
    ActionsBlock, DirectiveBlock, ReasoningBlock, Spanned, StartAgentBlock, TopicBlock,
    TopicSystemOverride,
};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::actions::actions_block;
use super::directives::{after_reasoning_block, before_reasoning_block};
use super::instructions::{dynamic_instructions, simple_instructions, static_instructions};
use super::primitives::{
    dedent, description_entry, indent, newline, skip_block_noise, spanned_ident, spanned_string,
    to_ast_span, ParserInput, Span,
};
use super::reasoning::reasoning_block;

/// Parse a topic/start_agent block entry.
#[derive(Clone)]
enum TopicEntry {
    Description(Spanned<String>),
    Reasoning(Spanned<ReasoningBlock>),
    Actions(Spanned<ActionsBlock>),
    BeforeReasoning(Spanned<DirectiveBlock>),
    AfterReasoning(Spanned<DirectiveBlock>),
    Label,
    System(Spanned<TopicSystemOverride>),
}

/// Parse a topic-level system override block.
fn topic_system_override<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<TopicSystemOverride>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::System)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            choice((simple_instructions(), static_instructions(), dynamic_instructions()))
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|instructions_list, e| {
            let instructions = instructions_list.into_iter().next();
            Spanned::new(TopicSystemOverride { instructions }, to_ast_span(e.span()))
        })
}

/// Parse a topic entry.
fn topic_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    TopicEntry,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        description_entry().map(TopicEntry::Description),
        topic_system_override().map(TopicEntry::System),
        before_reasoning_block().map(TopicEntry::BeforeReasoning),
        after_reasoning_block().map(TopicEntry::AfterReasoning),
        reasoning_block().map(TopicEntry::Reasoning),
        actions_block().map(TopicEntry::Actions),
        just(Token::Label)
            .ignore_then(just(Token::Colon))
            .ignore_then(spanned_string())
            .to(TopicEntry::Label),
    ))
}

/// Parse the content of a topic/start_agent block.
fn topic_content<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Vec<TopicEntry>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    newline()
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            topic_entry()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
}

/// Parse the start_agent block.
pub(crate) fn start_agent_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<StartAgentBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::StartAgent)
        .ignore_then(spanned_ident())
        .then_ignore(just(Token::Colon))
        .then(topic_content())
        .map_with(|(name, entries), e| {
            let mut block = StartAgentBlock {
                name,
                description: None,
                system: None,
                actions: None,
                before_reasoning: None,
                reasoning: None,
                after_reasoning: None,
            };

            for entry in entries {
                match entry {
                    TopicEntry::Description(d) => block.description = Some(d),
                    TopicEntry::System(s) => block.system = Some(s),
                    TopicEntry::Reasoning(r) => block.reasoning = Some(r),
                    TopicEntry::Actions(a) => block.actions = Some(a),
                    TopicEntry::BeforeReasoning(b) => block.before_reasoning = Some(b),
                    TopicEntry::AfterReasoning(a) => block.after_reasoning = Some(a),
                    TopicEntry::Label => {} // Ignored for start_agent
                }
            }

            Spanned::new(block, to_ast_span(e.span()))
        })
}

/// Parse a topic block.
pub(crate) fn topic_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<TopicBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Topic)
        .ignore_then(spanned_ident())
        .then_ignore(just(Token::Colon))
        .then(topic_content().labelled("topic content"))
        .labelled("topic block")
        .map_with(|(name, entries), e| {
            let mut block = TopicBlock {
                name,
                description: None,
                system: None,
                actions: None,
                before_reasoning: None,
                reasoning: None,
                after_reasoning: None,
            };

            for entry in entries {
                match entry {
                    TopicEntry::Description(d) => block.description = Some(d),
                    TopicEntry::System(s) => block.system = Some(s),
                    TopicEntry::Reasoning(r) => block.reasoning = Some(r),
                    TopicEntry::Actions(a) => block.actions = Some(a),
                    TopicEntry::BeforeReasoning(b) => block.before_reasoning = Some(b),
                    TopicEntry::AfterReasoning(a) => block.after_reasoning = Some(a),
                    TopicEntry::Label => {} // TODO: handle label
                }
            }

            Spanned::new(block, to_ast_span(e.span()))
        })
}
