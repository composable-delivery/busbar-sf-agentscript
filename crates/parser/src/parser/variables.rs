//! Variables block parser.
//!
//! Parses the `variables:` block containing variable declarations.

use crate::ast::{Expr, Reference, Spanned, Type, VariableDecl, VariableKind, VariablesBlock};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::expressions::{expr, reference, spanned_type};
use super::primitives::{
    dedent, description_entry, indent, newline, skip_block_noise, spanned_ident, to_ast_span,
    ParserInput, Span,
};

/// Parse a variable kind (mutable/linked).
fn variable_kind<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    VariableKind,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        just(Token::Mutable).to(VariableKind::Mutable),
        just(Token::Linked).to(VariableKind::Linked),
    ))
}

/// Helper enum for variable declaration entries.
#[derive(Clone)]
enum VarDeclEntry {
    Description(Spanned<String>),
    Source(Spanned<Reference>),
}

/// Parse a variable declaration nested entry (description or source).
fn var_decl_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    VarDeclEntry,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        description_entry().map(VarDeclEntry::Description),
        just(Token::Source)
            .ignore_then(just(Token::Colon))
            .ignore_then(reference())
            .map_with(|r, e| VarDeclEntry::Source(Spanned::new(r, to_ast_span(e.span())))),
    ))
}

/// Parse a variable declaration.
/// Variable syntax: name: mutable/linked type [= default]
///    [description: "..."]
///    [source: "..."]
pub(crate) fn variable_decl<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<VariableDecl>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    spanned_ident()
        .then_ignore(just(Token::Colon))
        .then(variable_kind())
        .then(spanned_type())
        .then(just(Token::Assign).ignore_then(expr()).or_not())
        .then(
            // Optional nested block with description/source
            newline()
                .ignore_then(skip_block_noise())
                .ignore_then(indent())
                .ignore_then(
                    var_decl_entry()
                        .separated_by(skip_block_noise())
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(skip_block_noise())
                .then_ignore(dedent())
                .or_not()
                .map(|opt| opt.unwrap_or_default()),
        )
        .validate(|((((name, kind), ty), default), entries), e, emitter| {
            // Validate: = None is only allowed for boolean types
            if let Some(ref def) = default {
                if matches!(def.node, Expr::None) && !matches!(ty.node, Type::Boolean) {
                    emitter.emit(Rich::custom(
                        e.span(),
                        format!(
                            "'= None' is only valid for boolean types, but '{}' has type '{:?}'",
                            name.node, ty.node
                        ),
                    ));
                }
            }
            ((((name, kind), ty), default), entries)
        })
        .map_with(|((((name, kind), ty), default), entries), e| {
            let mut description = None;
            let mut source = None;
            for entry in entries {
                match entry {
                    VarDeclEntry::Description(d) => description = Some(d),
                    VarDeclEntry::Source(s) => source = Some(s),
                }
            }
            Spanned::new(
                VariableDecl {
                    name,
                    kind,
                    ty,
                    default,
                    description,
                    source,
                },
                to_ast_span(e.span()),
            )
        })
}

/// Parse the variables block.
pub(crate) fn variables_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<VariablesBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Variables)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise()) // Skip comments before indent
        .ignore_then(indent())
        .ignore_then(
            variable_decl()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|variables, e| Spanned::new(VariablesBlock { variables }, to_ast_span(e.span())))
}
