//! Instructions parser.
//!
//! Parses instruction blocks including:
//! - Simple: `instructions: "..."`
//! - Static multiline: `instructions:|`
//! - Dynamic multiline: `instructions:->`
//!
//! NOTE: This module contains manual token parsing for multiline content.
//! This is a known architectural issue that should be replaced with proper
//! chumsky combinators when time permits.

use crate::ast::{BinOp, Expr, InstructionPart, Instructions, Spanned, UnaryOp};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::primitives::{
    dedent, indent, newline, skip_block_noise, spanned_string, to_ast_span, ParserInput, Span,
};

/// Parse simple instructions (single string).
pub(crate) fn simple_instructions<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Instructions>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Instructions)
        .ignore_then(just(Token::Colon))
        .ignore_then(spanned_string())
        .map(|s| {
            let span = s.span.clone();
            Spanned::new(Instructions::Simple(s.node), span)
        })
}

/// Collect all tokens until we hit a DEDENT at the current level.
/// Returns tokens that form the multiline content.
/// This properly handles nested INDENT/DEDENT pairs within the content.
fn collect_multiline_tokens<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Vec<(Token<'src>, Span)>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    newline()
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            recursive(|nested| {
                choice((
                    // Nested block: INDENT, content, DEDENT - include all tokens
                    indent()
                        .map_with(|_, e| (Token::Indent, e.span()))
                        .then(nested.clone().repeated().collect::<Vec<_>>().map(
                            |vecs: Vec<Vec<_>>| vecs.into_iter().flatten().collect::<Vec<_>>(),
                        ))
                        .then(dedent().map_with(|_, e| (Token::Dedent, e.span())))
                        .map(|((indent_tok, content), dedent_tok)| {
                            let mut tokens = vec![indent_tok];
                            tokens.extend(content);
                            tokens.push(dedent_tok);
                            tokens
                        }),
                    // Any other token except DEDENT (which closes our block)
                    any()
                        .filter(|t: &Token| !matches!(t, Token::Indent | Token::Dedent))
                        .map_with(|t, e| vec![(t.clone(), e.span())]),
                ))
            })
            .repeated()
            .collect::<Vec<_>>()
            .map(|vecs: Vec<Vec<_>>| vecs.into_iter().flatten().collect::<Vec<_>>()),
        )
        .then_ignore(dedent())
}

/// Parse static multiline instructions (:|).
pub(crate) fn static_instructions<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Instructions>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Instructions)
        .ignore_then(just(Token::ColonPipe))
        .ignore_then(collect_multiline_tokens())
        .map_with(|tokens, e| {
            let lines = parse_multiline_text_content(&tokens);
            Spanned::new(Instructions::Static(lines), to_ast_span(e.span()))
        })
}

/// Parse dynamic multiline instructions (:-> or : ->).
pub(crate) fn dynamic_instructions<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Instructions>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Instructions)
        .ignore_then(choice((
            just(Token::ColonArrow).ignored(),
            just(Token::Colon).ignore_then(just(Token::Arrow)).ignored(),
        )))
        .ignore_then(collect_multiline_tokens())
        .map_with(|tokens, e| {
            let parts = parse_instruction_parts(&tokens);
            Spanned::new(Instructions::Dynamic(parts), to_ast_span(e.span()))
        })
}

// ============================================================================
// Manual Token Parsing Functions
// NOTE: These should ideally be replaced with proper chumsky combinators
// ============================================================================

/// Parse multiline instruction text lines.
/// Handles both pipe-prefixed lines (legacy format) and plain indented text.
fn parse_multiline_text_content(tokens: &[(Token<'_>, Span)]) -> Vec<Spanned<String>> {
    let mut lines = Vec::new();
    let mut i = 0;

    // Check if any lines start with Pipe - if not, parse as plain text
    let has_pipes = tokens.iter().any(|(t, _)| matches!(t, Token::Pipe));

    if has_pipes {
        // Original pipe-prefixed format
        while i < tokens.len() {
            if matches!(&tokens[i].0, Token::Pipe) {
                let start_span = tokens[i].1;
                let mut line_text = String::new();
                i += 1;

                while i < tokens.len() && !matches!(&tokens[i].0, Token::Newline) {
                    match &tokens[i].0 {
                        Token::Ident(s) => {
                            if !line_text.is_empty() {
                                line_text.push(' ');
                            }
                            line_text.push_str(s);
                        }
                        Token::StringLit(s) => {
                            if !line_text.is_empty() {
                                line_text.push(' ');
                            }
                            line_text.push_str(s);
                        }
                        Token::NumberLit(n) => {
                            if !line_text.is_empty() {
                                line_text.push(' ');
                            }
                            line_text.push_str(&n.to_string());
                        }
                        other => {
                            let s = token_to_text(other);
                            if !s.is_empty() {
                                if !line_text.is_empty() && needs_space_before(&s) {
                                    line_text.push(' ');
                                }
                                line_text.push_str(&s);
                            }
                        }
                    }
                    i += 1;
                }

                let end_span = if i > 0 {
                    tokens[i - 1].1.end
                } else {
                    start_span.end
                };
                lines.push(Spanned::new(line_text, start_span.start..end_span));
            }
            i += 1;
        }
    } else {
        // Plain text format - no pipes, just indented text
        // Parse each line (separated by newlines)
        while i < tokens.len() {
            // Skip leading newlines
            while i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
                i += 1;
            }
            if i >= tokens.len() {
                break;
            }

            let start_span = tokens[i].1;
            let mut line_text = String::new();

            while i < tokens.len() && !matches!(&tokens[i].0, Token::Newline) {
                match &tokens[i].0 {
                    Token::Ident(s) => {
                        if !line_text.is_empty() {
                            line_text.push(' ');
                        }
                        line_text.push_str(s);
                    }
                    Token::StringLit(s) => {
                        if !line_text.is_empty() {
                            line_text.push(' ');
                        }
                        line_text.push_str(s);
                    }
                    Token::NumberLit(n) => {
                        if !line_text.is_empty() {
                            line_text.push(' ');
                        }
                        line_text.push_str(&n.to_string());
                    }
                    other => {
                        let s = token_to_text(other);
                        if !s.is_empty() {
                            if !line_text.is_empty() && needs_space_before(&s) {
                                line_text.push(' ');
                            }
                            line_text.push_str(&s);
                        }
                    }
                }
                i += 1;
            }

            if !line_text.is_empty() {
                let end_span = if i > 0 {
                    tokens[i - 1].1.end
                } else {
                    start_span.end
                };
                lines.push(Spanned::new(line_text, start_span.start..end_span));
            }
        }
    }

    lines
}

/// Parse a text line, extracting interpolations as separate parts.
/// Input: tokens from after Pipe to before Newline
/// Returns: Vec of Text and Interpolation parts
fn parse_text_line_with_interpolations(
    tokens: &[(Token<'_>, Span)],
    start_span: Span,
) -> Vec<Spanned<InstructionPart>> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut text_start = start_span.start;
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i].0 {
            Token::ExclBrace => {
                // Finish any accumulated text
                if !text.is_empty() {
                    parts.push(Spanned::new(
                        InstructionPart::Text(text.clone()),
                        text_start..tokens[i].1.start,
                    ));
                    text.clear();
                }

                let interp_start = tokens[i].1.start;
                i += 1;

                // Collect tokens until matching RBrace
                let expr_start = i;
                let mut brace_depth = 1;
                while i < tokens.len() && brace_depth > 0 {
                    match &tokens[i].0 {
                        Token::LBrace => brace_depth += 1,
                        Token::RBrace => {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }

                // Parse the collected tokens as an expression
                let expr_tokens = &tokens[expr_start..i];
                let expr = build_interpolation_expr(expr_tokens);

                let interp_end = if i < tokens.len() {
                    tokens[i].1.end
                } else {
                    tokens.last().map(|t| t.1.end).unwrap_or(interp_start)
                };

                parts.push(Spanned::new(
                    InstructionPart::Interpolation(expr.node),
                    interp_start..interp_end,
                ));

                // Skip the closing brace
                if i < tokens.len() && matches!(&tokens[i].0, Token::RBrace) {
                    i += 1;
                }

                // Update text_start for next text segment
                text_start = if i < tokens.len() {
                    tokens[i].1.start
                } else {
                    interp_end
                };
            }
            _ => {
                append_token_text(&mut text, &tokens[i].0);
                i += 1;
            }
        }
    }

    // Add any remaining text
    if !text.is_empty() {
        let end = tokens.last().map(|t| t.1.end).unwrap_or(text_start);
        parts.push(Spanned::new(InstructionPart::Text(text), text_start..end));
    }

    parts
}

/// Build an expression from interpolation tokens (similar to build_condition_expr).
fn build_interpolation_expr(tokens: &[(Token<'_>, Span)]) -> Spanned<Expr> {
    use crate::ast::Reference;

    if tokens.is_empty() {
        return Spanned::new(Expr::None, 0..0);
    }

    let start = tokens[0].1.start;
    let end = tokens.last().map(|t| t.1.end).unwrap_or(start);

    // Try to parse as a simple expression (reference, literal, or binary op)
    let mut expr_parts: Vec<Spanned<Expr>> = Vec::new();
    let mut ops: Vec<BinOp> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i].0 {
            Token::At => {
                let ref_start = tokens[i].1.start;
                i += 1;
                let mut ref_parts = Vec::new();
                while i < tokens.len() {
                    match &tokens[i].0 {
                        Token::Variables => {
                            ref_parts.push("variables".to_string());
                            i += 1;
                        }
                        Token::Actions => {
                            ref_parts.push("actions".to_string());
                            i += 1;
                        }
                        Token::Outputs => {
                            ref_parts.push("outputs".to_string());
                            i += 1;
                        }
                        Token::Topic => {
                            ref_parts.push("topic".to_string());
                            i += 1;
                        }
                        Token::Inputs => {
                            ref_parts.push("inputs".to_string());
                            i += 1;
                        }
                        Token::Ident(s) => {
                            ref_parts.push(s.to_string());
                            i += 1;
                        }
                        Token::Dot => {
                            i += 1;
                        }
                        _ => break,
                    }
                }
                let ref_end = if i > 0 {
                    tokens[i.saturating_sub(1)].1.end
                } else {
                    ref_start
                };
                let namespace = ref_parts.first().cloned().unwrap_or_default();
                let path = ref_parts.into_iter().skip(1).collect();
                expr_parts.push(Spanned::new(
                    Expr::Reference(Reference { namespace, path }),
                    ref_start..ref_end,
                ));
            }
            Token::StringLit(s) => {
                let span = tokens[i].1;
                expr_parts.push(Spanned::new(Expr::String(s.to_string()), span.start..span.end));
                i += 1;
            }
            Token::NumberLit(n) => {
                let span = tokens[i].1;
                expr_parts.push(Spanned::new(Expr::Number(*n), span.start..span.end));
                i += 1;
            }
            Token::True => {
                let span = tokens[i].1;
                expr_parts.push(Spanned::new(Expr::Bool(true), span.start..span.end));
                i += 1;
            }
            Token::False => {
                let span = tokens[i].1;
                expr_parts.push(Spanned::new(Expr::Bool(false), span.start..span.end));
                i += 1;
            }
            Token::None => {
                let span = tokens[i].1;
                expr_parts.push(Spanned::new(Expr::None, span.start..span.end));
                i += 1;
            }
            Token::Plus => {
                ops.push(BinOp::Add);
                i += 1;
            }
            Token::Minus => {
                // Could be subtraction or negation - for now treat as subtraction
                ops.push(BinOp::Sub);
                i += 1;
            }
            Token::And => {
                ops.push(BinOp::And);
                i += 1;
            }
            Token::Or => {
                ops.push(BinOp::Or);
                i += 1;
            }
            Token::Eq => {
                ops.push(BinOp::Eq);
                i += 1;
            }
            Token::Ne => {
                ops.push(BinOp::Ne);
                i += 1;
            }
            Token::Lt => {
                ops.push(BinOp::Lt);
                i += 1;
            }
            Token::Gt => {
                ops.push(BinOp::Gt);
                i += 1;
            }
            Token::Le => {
                ops.push(BinOp::Le);
                i += 1;
            }
            Token::Ge => {
                ops.push(BinOp::Ge);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if expr_parts.is_empty() {
        return Spanned::new(Expr::None, start..end);
    }

    // Build the expression tree (left-to-right, no precedence for now)
    let mut result = expr_parts.remove(0);
    for op in ops {
        if !expr_parts.is_empty() {
            let right = expr_parts.remove(0);
            let span = result.span.start..right.span.end;
            result = Spanned::new(
                Expr::BinOp {
                    left: Box::new(result),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }
    }

    result
}

/// Parse instruction parts from collected tokens.
/// The `if_depth` parameter tracks nesting level of `if` blocks.
/// Nested `if` statements (depth > 0) are not supported by the Salesforce platform.
fn parse_instruction_parts(tokens: &[(Token<'_>, Span)]) -> Vec<Spanned<InstructionPart>> {
    parse_instruction_parts_with_depth(tokens, 0)
}

fn parse_instruction_parts_with_depth(
    tokens: &[(Token<'_>, Span)],
    if_depth: usize,
) -> Vec<Spanned<InstructionPart>> {
    let mut parts = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i].0 {
            Token::Pipe => {
                let start_span = tokens[i].1;
                i += 1;

                // Collect tokens until newline
                let line_start = i;
                while i < tokens.len() && !matches!(&tokens[i].0, Token::Newline) {
                    i += 1;
                }
                let line_end = i;

                if i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
                    i += 1;
                }

                // Check for continuation (INDENT ... DEDENT)
                let mut continuation_tokens = Vec::new();
                if i < tokens.len() && matches!(&tokens[i].0, Token::Indent) {
                    i += 1;
                    let mut depth = 1;
                    while i < tokens.len() && depth > 0 {
                        match &tokens[i].0 {
                            Token::Indent => depth += 1,
                            Token::Dedent => depth -= 1,
                            _ => {
                                if depth > 0 {
                                    continuation_tokens.push(tokens[i].clone());
                                }
                            }
                        }
                        i += 1;
                    }
                }

                // Parse the main line with interpolations
                let line_tokens = &tokens[line_start..line_end];
                let mut line_parts = parse_text_line_with_interpolations(line_tokens, start_span);

                // If there's continuation, add it to the last text part or create a new one
                if !continuation_tokens.is_empty() {
                    let cont_parts = parse_text_line_with_interpolations(
                        &continuation_tokens,
                        continuation_tokens
                            .first()
                            .map(|t| t.1)
                            .unwrap_or(start_span),
                    );

                    // Add newline between main line and continuation
                    if let Some(last) = line_parts.last_mut() {
                        if let InstructionPart::Text(ref mut t) = last.node {
                            t.push('\n');
                        }
                    }

                    line_parts.extend(cont_parts);
                }

                parts.extend(line_parts);
            }
            Token::If => {
                if if_depth > 0 {
                    // Nested `if` in instructions is not supported by the platform.
                    // Skip the entire if block without parsing it.
                    i = skip_if_block(tokens, i);
                } else {
                    let (part, new_i) = parse_if_block(tokens, i);
                    if let Some(p) = part {
                        parts.push(p);
                    }
                    i = new_i;
                }
            }
            Token::Run => {
                i = skip_run_block(tokens, i);
            }
            Token::Comment(_) | Token::Newline => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    parts
}

/// Parse an if block from tokens.
fn parse_if_block(
    tokens: &[(Token<'_>, Span)],
    start: usize,
) -> (Option<Spanned<InstructionPart>>, usize) {
    let mut i = start;
    let start_span = tokens[i].1;

    i += 1; // Skip 'if'

    // Collect condition tokens until ':'
    let condition_start = i;
    while i < tokens.len() && !matches!(&tokens[i].0, Token::Colon) {
        i += 1;
    }
    let condition_end = i;

    let condition = build_condition_expr(&tokens[condition_start..condition_end]);

    // Skip ':' and newline
    if i < tokens.len() && matches!(&tokens[i].0, Token::Colon) {
        i += 1;
    }
    if i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
        i += 1;
    }

    // Expect INDENT
    if i >= tokens.len() || !matches!(&tokens[i].0, Token::Indent) {
        return (None, i);
    }
    i += 1;

    // Collect then-block tokens
    let then_tokens = collect_block_tokens(tokens, &mut i);
    // Use depth 1 to prevent nested if statements inside if bodies
    let then_parts = parse_instruction_parts_with_depth(&then_tokens, 1);

    // Check for else
    let else_parts = if i < tokens.len() && matches!(&tokens[i].0, Token::Else) {
        i += 1;
        if i < tokens.len() && matches!(&tokens[i].0, Token::Colon) {
            i += 1;
        }
        if i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
            i += 1;
        }
        if i < tokens.len() && matches!(&tokens[i].0, Token::Indent) {
            i += 1;
            let else_tokens = collect_block_tokens(tokens, &mut i);
            // Use depth 1 to prevent nested if statements inside else bodies
            Some(parse_instruction_parts_with_depth(&else_tokens, 1))
        } else {
            None
        }
    } else {
        None
    };

    let end_span = if i > start {
        tokens[i.saturating_sub(1)].1.end
    } else {
        start_span.end
    };

    (
        Some(Spanned::new(
            InstructionPart::Conditional {
                condition,
                then_parts,
                else_parts,
            },
            start_span.start..end_span,
        )),
        i,
    )
}

/// Build an expression from condition tokens.
fn build_condition_expr(tokens: &[(Token<'_>, Span)]) -> Spanned<Expr> {
    use crate::ast::Reference;

    if tokens.is_empty() {
        return Spanned::new(Expr::Bool(true), 0..0);
    }

    let start = tokens[0].1.start;
    let end = tokens.last().map(|t| t.1.end).unwrap_or(start);

    let mut expr_parts: Vec<Spanned<Expr>> = Vec::new();
    let mut ops: Vec<BinOp> = Vec::new();
    let mut i = 0;
    let mut negate_next = false;

    while i < tokens.len() {
        match &tokens[i].0 {
            Token::Not => {
                negate_next = true;
                i += 1;
            }
            Token::At => {
                let ref_start = tokens[i].1.start;
                i += 1;
                let mut ref_parts = Vec::new();
                while i < tokens.len() {
                    match &tokens[i].0 {
                        Token::Variables => {
                            ref_parts.push("variables".to_string());
                            i += 1;
                        }
                        Token::Actions => {
                            ref_parts.push("actions".to_string());
                            i += 1;
                        }
                        Token::Outputs => {
                            ref_parts.push("outputs".to_string());
                            i += 1;
                        }
                        Token::Ident(s) => {
                            ref_parts.push(s.to_string());
                            i += 1;
                        }
                        Token::Dot => {
                            i += 1;
                        }
                        _ => break,
                    }
                }
                let ref_end = if i > 0 {
                    tokens[i.saturating_sub(1)].1.end
                } else {
                    ref_start
                };
                let namespace = ref_parts.first().cloned().unwrap_or_default();
                let path = ref_parts.into_iter().skip(1).collect();
                let mut expr = Spanned::new(
                    Expr::Reference(Reference { namespace, path }),
                    ref_start..ref_end,
                );
                if negate_next {
                    expr = Spanned::new(
                        Expr::UnaryOp {
                            op: UnaryOp::Not,
                            operand: Box::new(expr),
                        },
                        ref_start..ref_end,
                    );
                    negate_next = false;
                }
                expr_parts.push(expr);
            }
            Token::And => {
                ops.push(BinOp::And);
                i += 1;
            }
            Token::Or => {
                ops.push(BinOp::Or);
                i += 1;
            }
            Token::Eq => {
                ops.push(BinOp::Eq);
                i += 1;
            }
            Token::Ne => {
                ops.push(BinOp::Ne);
                i += 1;
            }
            Token::Lt => {
                ops.push(BinOp::Lt);
                i += 1;
            }
            Token::Gt => {
                ops.push(BinOp::Gt);
                i += 1;
            }
            Token::Le => {
                ops.push(BinOp::Le);
                i += 1;
            }
            Token::Ge => {
                ops.push(BinOp::Ge);
                i += 1;
            }
            Token::StringLit(s) => {
                let span = tokens[i].1;
                let mut expr = Spanned::new(Expr::String(s.to_string()), span.start..span.end);
                if negate_next {
                    expr = Spanned::new(
                        Expr::UnaryOp {
                            op: UnaryOp::Not,
                            operand: Box::new(expr),
                        },
                        span.start..span.end,
                    );
                    negate_next = false;
                }
                expr_parts.push(expr);
                i += 1;
            }
            Token::NumberLit(n) => {
                let span = tokens[i].1;
                let mut expr = Spanned::new(Expr::Number(*n), span.start..span.end);
                if negate_next {
                    expr = Spanned::new(
                        Expr::UnaryOp {
                            op: UnaryOp::Not,
                            operand: Box::new(expr),
                        },
                        span.start..span.end,
                    );
                    negate_next = false;
                }
                expr_parts.push(expr);
                i += 1;
            }
            Token::True => {
                let span = tokens[i].1;
                let mut expr = Spanned::new(Expr::Bool(true), span.start..span.end);
                if negate_next {
                    expr = Spanned::new(
                        Expr::UnaryOp {
                            op: UnaryOp::Not,
                            operand: Box::new(expr),
                        },
                        span.start..span.end,
                    );
                    negate_next = false;
                }
                expr_parts.push(expr);
                i += 1;
            }
            Token::False => {
                let span = tokens[i].1;
                let mut expr = Spanned::new(Expr::Bool(false), span.start..span.end);
                if negate_next {
                    expr = Spanned::new(
                        Expr::UnaryOp {
                            op: UnaryOp::Not,
                            operand: Box::new(expr),
                        },
                        span.start..span.end,
                    );
                    negate_next = false;
                }
                expr_parts.push(expr);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if expr_parts.is_empty() {
        return Spanned::new(Expr::Bool(true), start..end);
    }

    let mut result = expr_parts.remove(0);
    for (i, op) in ops.into_iter().enumerate() {
        if i < expr_parts.len() {
            let right = expr_parts.remove(0);
            let span = result.span.start..right.span.end;
            result = Spanned::new(
                Expr::BinOp {
                    left: Box::new(result),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }
    }

    result
}

/// Collect tokens for an indented block.
fn collect_block_tokens<'a>(
    tokens: &'a [(Token<'a>, Span)],
    i: &mut usize,
) -> Vec<(Token<'a>, Span)> {
    let mut result = Vec::new();
    let mut depth = 1;

    while *i < tokens.len() && depth > 0 {
        match &tokens[*i].0 {
            Token::Indent => {
                depth += 1;
                result.push(tokens[*i].clone());
            }
            Token::Dedent => {
                depth -= 1;
                if depth > 0 {
                    result.push(tokens[*i].clone());
                }
            }
            _ => {
                result.push(tokens[*i].clone());
            }
        }
        *i += 1;
    }

    result
}

/// Skip a run statement with any nested indented block.
/// Skip an entire if/else block without parsing it.
/// Used to discard nested if statements that the platform doesn't support.
fn skip_if_block(tokens: &[(Token<'_>, Span)], start: usize) -> usize {
    let mut i = start;

    // Skip 'if' keyword and condition tokens until ':'
    while i < tokens.len() && !matches!(&tokens[i].0, Token::Colon) {
        i += 1;
    }
    // Skip ':' and newline
    if i < tokens.len() && matches!(&tokens[i].0, Token::Colon) {
        i += 1;
    }
    if i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
        i += 1;
    }
    // Skip indented then-block
    if i < tokens.len() && matches!(&tokens[i].0, Token::Indent) {
        i += 1;
        let mut depth = 1;
        while i < tokens.len() && depth > 0 {
            match &tokens[i].0 {
                Token::Indent => depth += 1,
                Token::Dedent => depth -= 1,
                _ => {}
            }
            i += 1;
        }
    }
    // Check for else block and skip it too
    if i < tokens.len() && matches!(&tokens[i].0, Token::Else) {
        i += 1;
        if i < tokens.len() && matches!(&tokens[i].0, Token::Colon) {
            i += 1;
        }
        if i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
            i += 1;
        }
        if i < tokens.len() && matches!(&tokens[i].0, Token::Indent) {
            i += 1;
            let mut depth = 1;
            while i < tokens.len() && depth > 0 {
                match &tokens[i].0 {
                    Token::Indent => depth += 1,
                    Token::Dedent => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
        }
    }

    i
}

fn skip_run_block(tokens: &[(Token<'_>, Span)], start: usize) -> usize {
    let mut i = start;

    while i < tokens.len() && !matches!(&tokens[i].0, Token::Newline) {
        i += 1;
    }

    if i < tokens.len() && matches!(&tokens[i].0, Token::Newline) {
        i += 1;
    }

    if i < tokens.len() && matches!(&tokens[i].0, Token::Indent) {
        i += 1;
        let mut depth = 1;
        while i < tokens.len() && depth > 0 {
            match &tokens[i].0 {
                Token::Indent => depth += 1,
                Token::Dedent => depth -= 1,
                _ => {}
            }
            i += 1;
        }
    }

    i
}

/// Append a token's text representation to a string.
fn append_token_text(text: &mut String, tok: &Token<'_>) {
    let s = token_to_text(tok);
    if !s.is_empty() {
        if !text.is_empty() && needs_space_before(&s) && needs_space_after_last(text) {
            text.push(' ');
        }
        text.push_str(&s);
    }
}

fn needs_space_before(s: &str) -> bool {
    !s.starts_with([':', '.', ',', ')', ']', '}', '!', '?'])
}

fn needs_space_after_last(s: &str) -> bool {
    !s.ends_with(['(', '[', '{', '@'])
}

/// Convert a single token to its text representation.
fn token_to_text(tok: &Token<'_>) -> String {
    match tok {
        Token::Ident(s) => s.to_string(),
        Token::StringLit(s) => format!("\"{}\"", s),
        Token::NumberLit(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Token::Newline => "\n".to_string(),
        Token::Colon => ":".to_string(),
        Token::Dot => ".".to_string(),
        Token::Comma => ",".to_string(),
        Token::Minus => "-".to_string(),
        Token::Plus => "+".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::LBrace => "{".to_string(),
        Token::RBrace => "}".to_string(),
        Token::ExclBrace => "{!".to_string(),
        Token::DoubleLBrace => "{{".to_string(),
        Token::DoubleBrace => "}}".to_string(),
        Token::At => "@".to_string(),
        Token::Slash => "/".to_string(),
        Token::Question => "?".to_string(),
        Token::Exclamation => "!".to_string(),
        Token::Dollar => "$".to_string(),
        Token::Percent => "%".to_string(),
        Token::Star => "*".to_string(),
        Token::Ampersand => "&".to_string(),
        Token::Semicolon => ";".to_string(),
        Token::Backtick => "`".to_string(),
        Token::Tilde => "~".to_string(),
        Token::Caret => "^".to_string(),
        Token::Backslash => "\\".to_string(),
        Token::Underscore => "_".to_string(),
        Token::Apostrophe => "'".to_string(),
        Token::Eq => "==".to_string(),
        Token::Ne => "!=".to_string(),
        Token::Le => "<=".to_string(),
        Token::Ge => ">=".to_string(),
        Token::Lt => "<".to_string(),
        Token::Gt => ">".to_string(),
        Token::Assign => "=".to_string(),
        Token::Ellipsis => "...".to_string(),
        Token::Arrow => "->".to_string(),
        Token::Pipe => "|".to_string(),
        Token::UnicodeText(s) => s.to_string(),
        // Keywords
        Token::If => "if".to_string(),
        Token::Else => "else".to_string(),
        Token::And => "and".to_string(),
        Token::Or => "or".to_string(),
        Token::Not => "not".to_string(),
        Token::True => "True".to_string(),
        Token::False => "False".to_string(),
        Token::None => "None".to_string(),
        Token::To => "to".to_string(),
        Token::With => "with".to_string(),
        Token::Set => "set".to_string(),
        Token::Run => "run".to_string(),
        Token::As => "as".to_string(),
        Token::Is => "is".to_string(),
        Token::Available => "available".to_string(),
        Token::When => "when".to_string(),
        Token::Transition => "transition".to_string(),
        Token::Variables => "variables".to_string(),
        Token::Actions => "actions".to_string(),
        Token::Outputs => "outputs".to_string(),
        Token::Inputs => "inputs".to_string(),
        Token::Topic => "topic".to_string(),
        Token::Description => "description".to_string(),
        Token::Source => "source".to_string(),
        Token::Target => "target".to_string(),
        Token::Label => "label".to_string(),
        Token::Config => "config".to_string(),
        Token::System => "system".to_string(),
        Token::Reasoning => "reasoning".to_string(),
        Token::Instructions => "instructions".to_string(),
        Token::Messages => "messages".to_string(),
        Token::Welcome => "welcome".to_string(),
        Token::Error => "error".to_string(),
        Token::Connection => "connection".to_string(),
        Token::Connections => "connections".to_string(),
        Token::Knowledge => "knowledge".to_string(),
        Token::Language => "language".to_string(),
        Token::StartAgent => "start_agent".to_string(),
        Token::BeforeReasoning => "before_reasoning".to_string(),
        Token::AfterReasoning => "after_reasoning".to_string(),
        Token::Mutable => "mutable".to_string(),
        Token::Linked => "linked".to_string(),
        Token::String => "string".to_string(),
        Token::Number => "number".to_string(),
        Token::Boolean => "boolean".to_string(),
        Token::Object => "object".to_string(),
        Token::List => "list".to_string(),
        Token::Date => "date".to_string(),
        Token::Timestamp => "timestamp".to_string(),
        Token::Currency => "currency".to_string(),
        Token::Id => "id".to_string(),
        Token::Datetime => "datetime".to_string(),
        Token::Time => "time".to_string(),
        Token::Integer => "integer".to_string(),
        Token::Long => "long".to_string(),
        Token::ColonPipe => ":|".to_string(),
        Token::ColonArrow => ":->".to_string(),
        Token::IsRequired => "is_required".to_string(),
        Token::IsDisplayable => "is_displayable".to_string(),
        Token::IsUsedByPlanner => "is_used_by_planner".to_string(),
        Token::ComplexDataTypeName => "complex_data_type_name".to_string(),
        Token::FilterFromAgent => "filter_from_agent".to_string(),
        Token::RequireUserConfirmation => "require_user_confirmation".to_string(),
        Token::IncludeInProgressIndicator => "include_in_progress_indicator".to_string(),
        Token::ProgressIndicatorMessage => "progress_indicator_message".to_string(),
        Token::Comment(_) | Token::Indent | Token::Dedent => String::new(),
    }
}
