//! Reasoning block parser.
//!
//! Parses reasoning blocks containing instructions and actions.

use crate::ast::{
    IfClause, ReasoningAction, ReasoningActionTarget, ReasoningBlock, Reference, RunClause,
    SetClause, Spanned, WithClause, WithValue,
};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::expressions::{expr, reference};
use super::instructions::{dynamic_instructions, simple_instructions, static_instructions};
use super::primitives::{
    dedent, description_entry, ident, indent, newline, skip_block_noise, spanned_ident, string_lit,
    to_ast_span, ParserInput, Span,
};

/// Parse a reasoning action target.
pub(crate) fn reasoning_action_target_parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ReasoningActionTarget>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        // @utils.transition to @topic.name
        just(Token::At)
            .ignore_then(ident().filter(|s| *s == "utils"))
            .ignore_then(just(Token::Dot))
            .ignore_then(just(Token::Transition))
            .ignore_then(just(Token::To))
            .ignore_then(reference())
            .map_with(|target, e| {
                Spanned::new(ReasoningActionTarget::TransitionTo(target), to_ast_span(e.span()))
            }),
        // @utils.escalate
        just(Token::At)
            .ignore_then(ident().filter(|s| *s == "utils"))
            .ignore_then(just(Token::Dot))
            .ignore_then(ident().filter(|s| *s == "escalate"))
            .map_with(|_, e| Spanned::new(ReasoningActionTarget::Escalate, to_ast_span(e.span()))),
        // @utils.setVariables
        just(Token::At)
            .ignore_then(ident().filter(|s| *s == "utils"))
            .ignore_then(just(Token::Dot))
            .ignore_then(ident().filter(|s| *s == "setVariables"))
            .map_with(|_, e| {
                Spanned::new(ReasoningActionTarget::SetVariables, to_ast_span(e.span()))
            }),
        // @topic.name (topic delegation)
        just(Token::At)
            .ignore_then(just(Token::Topic))
            .ignore_then(just(Token::Dot))
            .ignore_then(ident())
            .map_with(|name, e| {
                let r = Reference::new("topic", vec![name.to_string()]);
                Spanned::new(ReasoningActionTarget::TopicDelegate(r), to_ast_span(e.span()))
            }),
        // @actions.name
        reference()
            .filter(|r| r.namespace == "actions")
            .map_with(|r, e| Spanned::new(ReasoningActionTarget::Action(r), to_ast_span(e.span()))),
    ))
}

/// Parse a with clause: `with param=value`
pub(crate) fn with_clause<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<WithClause>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    // Parameter name can be an identifier, string literal, or certain keywords used as param names
    let param_name = choice((
        ident().map(|s| s.to_string()),
        string_lit().map(|s| s.to_string()),
        just(Token::Description).to("description".to_string()),
    ));

    just(Token::With)
        .ignore_then(param_name)
        .then_ignore(just(Token::Assign))
        .then(expr().map(|e| WithValue::Expr(e.node)))
        .map_with(|(param, value), e| {
            Spanned::new(
                WithClause {
                    param: Spanned::new(param.to_string(), to_ast_span(e.span())),
                    value: Spanned::new(value, to_ast_span(e.span())),
                },
                to_ast_span(e.span()),
            )
        })
}

/// Parse a set clause: `set @variables.x = <expr>`
/// The source can be a reference, literal, or expression.
pub(crate) fn set_clause<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<SetClause>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Set)
        .ignore_then(reference())
        .then_ignore(just(Token::Assign))
        .then(expr())
        .map_with(|(target, source), e| {
            Spanned::new(
                SetClause {
                    target: Spanned::new(target, to_ast_span(e.span())),
                    source,
                },
                to_ast_span(e.span()),
            )
        })
}

/// Helper enum for run clause entries.
#[derive(Clone)]
pub(crate) enum RunClauseEntry {
    With(Spanned<WithClause>),
    Set(Spanned<SetClause>),
}

/// Parse the nested block of a run clause.
fn run_clause_nested_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Vec<RunClauseEntry>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((with_clause().map(RunClauseEntry::With), set_clause().map(RunClauseEntry::Set)))
        .separated_by(skip_block_noise())
        .allow_trailing()
        .collect::<Vec<_>>()
}

/// Parse a run clause: `run @actions.x` with optional nested clauses.
pub(crate) fn run_clause<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<RunClause>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    let nested_block = newline()
        .ignore_then(indent())
        .ignore_then(run_clause_nested_block())
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .or_not()
        .map(|opt| opt.unwrap_or_default())
        .boxed();

    just(Token::Run)
        .ignore_then(reference())
        .then(nested_block)
        .map_with(|(action, entries), e| {
            let mut with_clauses = Vec::new();
            let mut set_clauses = Vec::new();
            for entry in entries {
                match entry {
                    RunClauseEntry::With(w) => with_clauses.push(w),
                    RunClauseEntry::Set(s) => set_clauses.push(s),
                }
            }
            Spanned::new(
                RunClause {
                    action: Spanned::new(action, to_ast_span(e.span())),
                    with_clauses,
                    set_clauses,
                },
                to_ast_span(e.span()),
            )
        })
}

/// Helper enum for reasoning action entries.
#[derive(Clone)]
enum ReasoningActionEntry {
    Description(Spanned<String>),
    With(Spanned<WithClause>),
    Set(Spanned<SetClause>),
    AvailableWhen(Spanned<crate::ast::Expr>),
    Run(Spanned<RunClause>),
    Transition(Spanned<Reference>),
    If(Spanned<IfClause>),
}

/// Parse a reasoning action definition.
pub(crate) fn reasoning_action<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ReasoningAction>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    spanned_ident()
        .then_ignore(just(Token::Colon))
        .then(reasoning_action_target_parser())
        .labelled("reasoning action")
        .then(
            newline()
                .ignore_then(indent())
                .ignore_then(
                    choice((
                        description_entry().map(ReasoningActionEntry::Description),
                        with_clause().map(ReasoningActionEntry::With),
                        set_clause().map(ReasoningActionEntry::Set),
                        just(Token::Available)
                            .ignore_then(just(Token::When))
                            .ignore_then(expr())
                            .map(ReasoningActionEntry::AvailableWhen),
                        run_clause().map(ReasoningActionEntry::Run),
                        // if <condition>: transition to <ref>
                        just(Token::If)
                            .ignore_then(expr())
                            .then_ignore(just(Token::Colon))
                            .then(
                                newline()
                                    .ignore_then(indent())
                                    .ignore_then(
                                        just(Token::Transition)
                                            .ignore_then(just(Token::To))
                                            .ignore_then(reference())
                                            .map_with(|target, e| {
                                                Some(Spanned::new(target, to_ast_span(e.span())))
                                            }),
                                    )
                                    .then_ignore(skip_block_noise())
                                    .then_ignore(dedent()),
                            )
                            .map_with(|(condition, transition), e| {
                                ReasoningActionEntry::If(Spanned::new(
                                    IfClause {
                                        condition,
                                        transition,
                                    },
                                    to_ast_span(e.span()),
                                ))
                            }),
                        just(Token::Transition)
                            .ignore_then(just(Token::To))
                            .ignore_then(reference())
                            .map_with(|target, e| {
                                ReasoningActionEntry::Transition(Spanned::new(
                                    target,
                                    to_ast_span(e.span()),
                                ))
                            }),
                    ))
                    .separated_by(skip_block_noise())
                    .allow_trailing()
                    .collect::<Vec<_>>(),
                )
                .then_ignore(skip_block_noise())
                .then_ignore(dedent())
                .or_not()
                .map(|opt| opt.unwrap_or_default()),
        )
        .map_with(|((name, target), entries), e| {
            let mut action = ReasoningAction {
                name,
                target,
                description: None,
                available_when: None,
                with_clauses: Vec::new(),
                set_clauses: Vec::new(),
                run_clauses: Vec::new(),
                if_clauses: Vec::new(),
                transition: None,
            };

            for entry in entries {
                match entry {
                    ReasoningActionEntry::Description(d) => action.description = Some(d),
                    ReasoningActionEntry::With(w) => action.with_clauses.push(w),
                    ReasoningActionEntry::Set(s) => action.set_clauses.push(s),
                    ReasoningActionEntry::AvailableWhen(e) => action.available_when = Some(e),
                    ReasoningActionEntry::Run(r) => action.run_clauses.push(r),
                    ReasoningActionEntry::Transition(t) => action.transition = Some(t),
                    ReasoningActionEntry::If(i) => action.if_clauses.push(i),
                }
            }

            Spanned::new(action, to_ast_span(e.span()))
        })
}

/// Helper enum for reasoning block entries.
#[derive(Clone)]
enum ReasoningEntry {
    Instructions(Spanned<crate::ast::Instructions>),
    Actions(Vec<Spanned<ReasoningAction>>),
}

/// Parse the reasoning block.
pub(crate) fn reasoning_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ReasoningBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Reasoning)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            choice((
                simple_instructions().map(ReasoningEntry::Instructions),
                static_instructions().map(ReasoningEntry::Instructions),
                dynamic_instructions().map(ReasoningEntry::Instructions),
                just(Token::Actions)
                    .map_with(|_, e| e.span()) // Capture the span of 'actions'
                    .then_ignore(just(Token::Colon))
                    .then_ignore(newline())
                    .then_ignore(skip_block_noise())
                    .then(
                        just(Token::Dedent)
                            .rewind()
                            .to(None)
                            .or(
                                indent()
                                    .ignore_then(
                                        reasoning_action()
                                            .separated_by(skip_block_noise())
                                            .allow_trailing()
                                            .collect::<Vec<_>>(),
                                    )
                                    .then_ignore(skip_block_noise())
                                    .then_ignore(dedent())
                                    .map(Some)
                            )
                    )
                    .validate(|(actions_span, entries), _, emitter| {
                        if entries.is_none() {
                            emitter.emit(Rich::custom(actions_span, "reasoning actions block cannot be empty"));
                        }
                        entries.unwrap_or_default()
                    })
                    .labelled("reasoning actions")
                    .map(ReasoningEntry::Actions),
            ))
            .separated_by(skip_block_noise())
            .allow_trailing()
            .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .labelled("reasoning block")
        .map_with(|entries, e| {
            let mut block = ReasoningBlock {
                instructions: None,
                actions: None,
            };

            for entry in entries {
                match entry {
                    ReasoningEntry::Instructions(i) => block.instructions = Some(i),
                    ReasoningEntry::Actions(a) => {
                        block.actions = Some(Spanned::new(a, to_ast_span(e.span())))
                    }
                }
            }

            Spanned::new(block, to_ast_span(e.span()))
        })
}
