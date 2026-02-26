//! Expression, reference, and type parsers.
//!
//! This module contains parsers for references (@namespace.path),
//! type specifications (string, number, list[T], etc.), and
//! expressions with operators.

use crate::ast::{BinOp, Expr, Reference, Spanned, Type, UnaryOp};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::primitives::{number_lit, string_lit, to_ast_span, ParserInput, Span};

/// Parse a reference: @namespace.path.to.something
pub fn reference<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Reference,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::At)
        .ignore_then(select! {
            Token::Ident(s) => s.to_string(),
            Token::Variables => "variables".to_string(),
            Token::Actions => "actions".to_string(),
            Token::Outputs => "outputs".to_string(),
            Token::Topic => "topic".to_string(),
            Token::Inputs => "inputs".to_string(),
        })
        .then(
            just(Token::Dot)
                .ignore_then(select! {
                    Token::Ident(s) => s.to_string(),
                    Token::Transition => "transition".to_string(),
                    Token::To => "to".to_string(),
                })
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(namespace, path)| Reference { namespace, path })
}

/// Parse a spanned reference.
#[allow(dead_code)]
pub fn spanned_reference<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Reference>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    reference().map_with(|r, e| Spanned::new(r, to_ast_span(e.span())))
}

/// Parse a type specification.
fn type_spec<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Type,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    recursive(|type_| {
        let simple_type = select! {
            Token::String => Type::String,
            Token::Number => Type::Number,
            Token::Boolean => Type::Boolean,
            Token::Object => Type::Object,
            Token::Date => Type::Date,
            Token::Timestamp => Type::Timestamp,
            Token::Currency => Type::Currency,
            Token::Id => Type::Id,
            Token::Datetime => Type::Datetime,
            Token::Time => Type::Time,
            Token::Integer => Type::Integer,
            Token::Long => Type::Long,
        };

        // list[inner_type]
        let list_type = just(Token::List)
            .ignore_then(just(Token::LBracket))
            .ignore_then(type_.clone())
            .then_ignore(just(Token::RBracket))
            .map(|inner| Type::List(Box::new(inner)));

        choice((list_type, simple_type))
    })
}

/// Parse a spanned type.
pub fn spanned_type<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Type>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    type_spec().map_with(|t, e| Spanned::new(t, to_ast_span(e.span())))
}

/// Parse an identifier for property access.
fn property_ident<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<String>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    select! {
        Token::Ident(s) => s.to_string(),
        Token::Variables => "variables".to_string(),
        Token::Actions => "actions".to_string(),
        Token::Outputs => "outputs".to_string(),
        Token::Topic => "topic".to_string(),
        Token::Description => "description".to_string(),
        Token::Label => "label".to_string(),
        Token::Source => "source".to_string(),
        Token::Target => "target".to_string(),
        Token::Error => "error".to_string(),
        Token::String => "string".to_string(),
        Token::Number => "number".to_string(),
        Token::Boolean => "boolean".to_string(),
        Token::Object => "object".to_string(),
        Token::List => "list".to_string(),
        Token::Id => "id".to_string(),
    }
    .map_with(|s, e| Spanned::new(s, to_ast_span(e.span())))
}

/// Parse an expression.
pub fn expr<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Expr>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    recursive(|expr| {
        // Atomic expressions (highest precedence)
        // NOTE: Salesforce AgentScript does NOT support inline object literals like {} or {key: value}
        // Use None for empty objects and proper action calls for object creation
        let atom = choice((
            // Literals
            string_lit().map(|s| Expr::String(s.to_string())),
            number_lit().map(Expr::Number),
            just(Token::True).to(Expr::Bool(true)),
            just(Token::False).to(Expr::Bool(false)),
            just(Token::None).to(Expr::None),
            // Reference
            reference().map(Expr::Reference),
            // Parenthesized expression
            expr.clone()
                .delimited_by(just(Token::LParen), just(Token::RParen))
                .map(|e: Spanned<Expr>| e.node),
            // Empty list
            just(Token::LBracket)
                .ignore_then(just(Token::RBracket))
                .to(Expr::List(Vec::new())),
            // Non-empty list
            expr.clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBracket), just(Token::RBracket))
                .map(Expr::List),
        ))
        .map_with(|e, extra| Spanned::new(e, to_ast_span(extra.span())));

        // Postfix operators: .field and [index]
        let postfix = atom.clone().foldl_with(
            choice((
                // Property access: .field
                just(Token::Dot)
                    .ignore_then(property_ident())
                    .map(|f| (true, f, None)),
                // Index access: [expr]
                expr.clone()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(|idx| (false, Spanned::new(String::new(), 0..0), Some(idx))),
            ))
            .repeated(),
            |obj, (is_prop, field, idx), e| {
                let span = obj.span.start..e.span().end;
                if is_prop {
                    Spanned::new(
                        Expr::Property {
                            object: Box::new(obj),
                            field,
                        },
                        span,
                    )
                } else {
                    Spanned::new(
                        Expr::Index {
                            object: Box::new(obj),
                            index: Box::new(idx.unwrap()),
                        },
                        span,
                    )
                }
            },
        );

        // Unary operators
        let unary =
            choice((just(Token::Not).to(UnaryOp::Not), just(Token::Minus).to(UnaryOp::Neg)))
                .repeated()
                .foldr_with(postfix.clone(), |op, expr, e| {
                    let span = expr.span.start..e.span().end;
                    Spanned::new(
                        Expr::UnaryOp {
                            op,
                            operand: Box::new(expr),
                        },
                        span,
                    )
                });

        // Comparison operators
        let comparison_op = choice((
            just(Token::Eq).to(BinOp::Eq),
            just(Token::Ne).to(BinOp::Ne),
            just(Token::Le).to(BinOp::Le),
            just(Token::Ge).to(BinOp::Ge),
            just(Token::Lt).to(BinOp::Lt),
            just(Token::Gt).to(BinOp::Gt),
        ));

        // Arithmetic operators (higher precedence)
        let add_op = choice((just(Token::Plus).to(BinOp::Add), just(Token::Minus).to(BinOp::Sub)));

        // Build expression with precedence using foldl
        let sum = unary
            .clone()
            .foldl_with(add_op.then(unary).repeated(), |l, (op, r), e| {
                let span = l.span.start..e.span().end;
                Spanned::new(
                    Expr::BinOp {
                        left: Box::new(l),
                        op,
                        right: Box::new(r),
                    },
                    span,
                )
            });

        let comparison =
            sum.clone()
                .foldl_with(comparison_op.then(sum).repeated(), |l, (op, r), e| {
                    let span = l.span.start..e.span().end;
                    Spanned::new(
                        Expr::BinOp {
                            left: Box::new(l),
                            op,
                            right: Box::new(r),
                        },
                        span,
                    )
                });

        // Logical AND
        let and_expr = comparison.clone().foldl_with(
            just(Token::And).ignore_then(comparison).repeated(),
            |l, r, e| {
                let span = l.span.start..e.span().end;
                Spanned::new(
                    Expr::BinOp {
                        left: Box::new(l),
                        op: BinOp::And,
                        right: Box::new(r),
                    },
                    span,
                )
            },
        );

        // Logical OR
        let or_expr = and_expr.clone().foldl_with(
            just(Token::Or).ignore_then(and_expr).repeated(),
            |l, r, e| {
                let span = l.span.start..e.span().end;
                Spanned::new(
                    Expr::BinOp {
                        left: Box::new(l),
                        op: BinOp::Or,
                        right: Box::new(r),
                    },
                    span,
                )
            },
        );

        // Box or_expr to break the type chain and prevent exponential compile times
        // See: https://github.com/zesterer/chumsky/discussions/396
        let or_expr = or_expr.boxed();

        // Ternary expression: value if condition else other
        or_expr
            .clone()
            .then(
                just(Token::If)
                    .ignore_then(or_expr.clone())
                    .then_ignore(just(Token::Else))
                    .then(or_expr)
                    .or_not(),
            )
            .map_with(|(then_expr, rest), e| match rest {
                Some((cond, else_expr)) => {
                    let span = then_expr.span.start..e.span().end;
                    Spanned::new(
                        Expr::Ternary {
                            condition: Box::new(cond),
                            then_expr: Box::new(then_expr),
                            else_expr: Box::new(else_expr),
                        },
                        span,
                    )
                }
                None => then_expr,
            })
            .boxed()
    })
}
