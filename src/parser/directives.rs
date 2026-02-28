//! Directive blocks parser.
//!
//! Parses `before_reasoning:` and `after_reasoning:` directive blocks.

use crate::ast::{DirectiveBlock, Spanned, Stmt};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::expressions::{expr, reference};
use super::primitives::{
    dedent, indent, newline, skip_block_noise, to_ast_span, ParserInput, Span,
};
use super::reasoning::{set_clause, with_clause};

/// Helper enum for run statement entries.
#[derive(Clone)]
enum RunEntry {
    With(Spanned<crate::ast::WithClause>),
    Set(Spanned<crate::ast::SetClause>),
}

/// Parse a statement in a directive block.
fn directive_stmt<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Stmt>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    recursive(|stmt| {
        choice((
            // if condition: with optional else:
            just(Token::If)
                .ignore_then(expr())
                .then_ignore(just(Token::Colon))
                .then(
                    newline()
                        .ignore_then(indent())
                        .ignore_then(
                            stmt.clone()
                                .separated_by(skip_block_noise())
                                .allow_trailing()
                                .collect::<Vec<_>>(),
                        )
                        .then_ignore(skip_block_noise())
                        .then_ignore(dedent()),
                )
                .then(
                    newline()
                        .or_not()
                        .ignore_then(skip_block_noise())
                        .ignore_then(just(Token::Else))
                        .ignore_then(just(Token::Colon))
                        .ignore_then(newline())
                        .ignore_then(indent())
                        .ignore_then(
                            stmt.clone()
                                .separated_by(skip_block_noise())
                                .allow_trailing()
                                .collect::<Vec<_>>(),
                        )
                        .then_ignore(skip_block_noise())
                        .then_ignore(dedent())
                        .or_not(),
                )
                .labelled("if statement")
                .map_with(|((condition, then_block), else_block), e| {
                    Spanned::new(
                        Stmt::If {
                            condition,
                            then_block,
                            else_block,
                        },
                        to_ast_span(e.span()),
                    )
                }),
            // set @ref = expr
            just(Token::Set)
                .ignore_then(reference())
                .then_ignore(just(Token::Assign))
                .then(expr())
                .map_with(|(target, value), e| {
                    Spanned::new(
                        Stmt::Set {
                            target: Spanned::new(target, to_ast_span(e.span())),
                            value,
                        },
                        to_ast_span(e.span()),
                    )
                }),
            // run @ref with optional clauses
            just(Token::Run)
                .ignore_then(reference())
                .then(
                    newline()
                        .ignore_then(indent())
                        .ignore_then(
                            choice((
                                with_clause().map(RunEntry::With),
                                set_clause().map(RunEntry::Set),
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
                .map_with(|(action, entries), e| {
                    let mut with_clauses = Vec::new();
                    let mut set_clauses = Vec::new();
                    for entry in entries {
                        match entry {
                            RunEntry::With(w) => with_clauses.push(w),
                            RunEntry::Set(s) => set_clauses.push(s),
                        }
                    }
                    Spanned::new(
                        Stmt::Run {
                            action: Spanned::new(action, to_ast_span(e.span())),
                            with_clauses,
                            set_clauses,
                        },
                        to_ast_span(e.span()),
                    )
                }),
        ))
    })
}

/// Parse the before_reasoning block.
pub(crate) fn before_reasoning_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<DirectiveBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::BeforeReasoning)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            directive_stmt()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .labelled("before_reasoning block")
        .map_with(|statements, e| {
            Spanned::new(DirectiveBlock { statements }, to_ast_span(e.span()))
        })
}

/// Parse the after_reasoning block.
pub(crate) fn after_reasoning_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<DirectiveBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::AfterReasoning)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            directive_stmt()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .labelled("after_reasoning block")
        .map_with(|statements, e| {
            Spanned::new(DirectiveBlock { statements }, to_ast_span(e.span()))
        })
}
