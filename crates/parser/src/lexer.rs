//! Lexer for AgentScript source code.
//!
//! This module provides tokenization of AgentScript source, handling:
//!
//! - Keywords (`config`, `topic`, `reasoning`, etc.)
//! - Operators (`==`, `!=`, `and`, `or`, etc.)
//! - Literals (strings, numbers, booleans)
//! - References (`@variables.name`)
//! - Indentation tracking (INDENT/DEDENT tokens)
//!
//! # Indentation Handling
//!
//! AgentScript uses significant whitespace like Python. The lexer tracks
//! indentation levels and emits `INDENT`/`DEDENT` tokens when the level
//! changes. This is handled by [`lex_with_indentation()`].
//!
//! # Example
//!
//! ```rust
//! use busbar_sf_agentscript_parser::lexer::{lexer, lex_with_indentation, Token};
//! use chumsky::prelude::*;  // For Parser trait
//!
//! // Basic tokenization
//! let tokens = lexer().parse("config:").into_result().unwrap();
//! assert_eq!(tokens[0].0, Token::Config);
//!
//! // With indentation tracking
//! let source = "config:\n   agent_name: \"Test\"";
//! let tokens = lex_with_indentation(source).unwrap();
//! // Contains INDENT token after the newline
//! ```
//!
//! # Token Types
//!
//! | Category | Examples |
//! |----------|----------|
//! | Keywords | `config`, `variables`, `topic`, `reasoning` |
//! | Types | `string`, `number`, `boolean`, `list` |
//! | Operators | `==`, `!=`, `and`, `or`, `not` |
//! | Literals | `"text"`, `42`, `True`, `False`, `None` |
//! | Punctuation | `:`, `.`, `@`, `\|`, `->` |
//! | Indentation | `INDENT`, `DEDENT`, `Newline` |

use chumsky::prelude::*;

/// A token in AgentScript.
///
/// Tokens are the atomic units produced by the lexer. Each token represents
/// a meaningful element of the source code.
#[derive(Clone, Debug, PartialEq)]
pub enum Token<'src> {
    // Keywords (block types)
    Config,
    Variables,
    System,
    StartAgent,
    Topic,
    Actions,
    Inputs,
    Outputs,
    Target,
    Reasoning,
    Instructions,
    BeforeReasoning,
    AfterReasoning,
    Messages,
    Welcome,
    Error,
    Connection,  // singular: connection <name>:
    Connections, // legacy plural form (for error messages)
    Knowledge,
    Language,

    // Variable keywords
    Mutable,
    Linked,
    Description,
    Source,
    Label,

    // ParamDef keywords
    IsRequired,
    IsDisplayable,
    IsUsedByPlanner,
    ComplexDataTypeName,
    FilterFromAgent,

    // ActionDef keywords
    RequireUserConfirmation,
    IncludeInProgressIndicator,
    ProgressIndicatorMessage,

    // Type keywords
    String,
    Number,
    Boolean,
    Object,
    List,
    Date,
    Timestamp,
    Currency,
    Id,
    Datetime,
    Time,
    Integer,
    Long,

    // Statement keywords
    If,
    Else,
    Run,
    With,
    Set,
    To,
    As,
    Transition,
    Available,
    When,

    // Literals
    True,
    False,
    None,

    // Operators
    Eq,     // ==
    Ne,     // !=
    Lt,     // <
    Gt,     // >
    Le,     // <=
    Ge,     // >=
    Assign, // =
    Is,     // is
    Not,    // not
    And,    // and
    Or,     // or
    Plus,   // +
    Minus,  // -

    // Punctuation
    Colon,        // :
    Dot,          // .
    Comma,        // ,
    At,           // @
    Pipe,         // |
    Arrow,        // ->
    ColonPipe,    // :|
    ColonArrow,   // :->
    LParen,       // (
    RParen,       // )
    LBracket,     // [
    RBracket,     // ]
    LBrace,       // {
    RBrace,       // }
    ExclBrace,    // {!
    DoubleLBrace, // {{
    DoubleBrace,  // }}
    Ellipsis,     // ...

    // Additional text punctuation (appears in instruction content)
    Slash,       // /
    Question,    // ?
    Exclamation, // !
    Dollar,      // $
    Percent,     // %
    Star,        // *
    Ampersand,   // &
    Semicolon,   // ;
    Backtick,    // `
    Tilde,       // ~
    Caret,       // ^
    Backslash,   // \
    Underscore,  // _
    Apostrophe,  // ' (single quote in text, not a string delimiter)

    // Unicode text (emojis, special symbols, non-ASCII characters)
    UnicodeText(&'src str),

    // Identifier
    Ident(&'src str),

    // String literal (content without quotes)
    StringLit(&'src str),

    // Number literal
    NumberLit(f64),

    // Comment (text without #)
    Comment(&'src str),

    // Newline (preserved for indentation tracking)
    Newline,

    // Indentation tokens (added by post-processing)
    Indent, // Indentation increased
    Dedent, // Indentation decreased
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Config => write!(f, "config"),
            Token::Variables => write!(f, "variables"),
            Token::System => write!(f, "system"),
            Token::StartAgent => write!(f, "start_agent"),
            Token::Topic => write!(f, "topic"),
            Token::Actions => write!(f, "actions"),
            Token::Inputs => write!(f, "inputs"),
            Token::Outputs => write!(f, "outputs"),
            Token::Target => write!(f, "target"),
            Token::Reasoning => write!(f, "reasoning"),
            Token::Instructions => write!(f, "instructions"),
            Token::BeforeReasoning => write!(f, "before_reasoning"),
            Token::AfterReasoning => write!(f, "after_reasoning"),
            Token::Messages => write!(f, "messages"),
            Token::Welcome => write!(f, "welcome"),
            Token::Error => write!(f, "error"),
            Token::Connection => write!(f, "connection"),
            Token::Connections => write!(f, "connections"),
            Token::Knowledge => write!(f, "knowledge"),
            Token::Language => write!(f, "language"),
            Token::Mutable => write!(f, "mutable"),
            Token::Linked => write!(f, "linked"),
            Token::Description => write!(f, "description"),
            Token::Source => write!(f, "source"),
            Token::Label => write!(f, "label"),
            Token::IsRequired => write!(f, "is_required"),
            Token::IsDisplayable => write!(f, "is_displayable"),
            Token::IsUsedByPlanner => write!(f, "is_used_by_planner"),
            Token::ComplexDataTypeName => write!(f, "complex_data_type_name"),
            Token::FilterFromAgent => write!(f, "filter_from_agent"),
            Token::RequireUserConfirmation => write!(f, "require_user_confirmation"),
            Token::IncludeInProgressIndicator => write!(f, "include_in_progress_indicator"),
            Token::ProgressIndicatorMessage => write!(f, "progress_indicator_message"),
            Token::String => write!(f, "string"),
            Token::Number => write!(f, "number"),
            Token::Boolean => write!(f, "boolean"),
            Token::Object => write!(f, "object"),
            Token::List => write!(f, "list"),
            Token::Date => write!(f, "date"),
            Token::Timestamp => write!(f, "timestamp"),
            Token::Currency => write!(f, "currency"),
            Token::Id => write!(f, "id"),
            Token::Datetime => write!(f, "datetime"),
            Token::Time => write!(f, "time"),
            Token::Integer => write!(f, "integer"),
            Token::Long => write!(f, "long"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Run => write!(f, "run"),
            Token::With => write!(f, "with"),
            Token::Set => write!(f, "set"),
            Token::To => write!(f, "to"),
            Token::As => write!(f, "as"),
            Token::Transition => write!(f, "transition"),
            Token::Available => write!(f, "available"),
            Token::When => write!(f, "when"),
            Token::True => write!(f, "True"),
            Token::False => write!(f, "False"),
            Token::None => write!(f, "None"),
            Token::Eq => write!(f, "=="),
            Token::Ne => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::Le => write!(f, "<="),
            Token::Ge => write!(f, ">="),
            Token::Assign => write!(f, "="),
            Token::Is => write!(f, "is"),
            Token::Not => write!(f, "not"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Colon => write!(f, ":"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::At => write!(f, "@"),
            Token::Pipe => write!(f, "|"),
            Token::Arrow => write!(f, "->"),
            Token::ColonPipe => write!(f, ":|"),
            Token::ColonArrow => write!(f, ":->"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::ExclBrace => write!(f, "{{!"),
            Token::DoubleLBrace => write!(f, "{{{{"),
            Token::DoubleBrace => write!(f, "}}}}"),
            Token::Ellipsis => write!(f, "..."),
            Token::Slash => write!(f, "/"),
            Token::Question => write!(f, "?"),
            Token::Exclamation => write!(f, "!"),
            Token::Dollar => write!(f, "$"),
            Token::Percent => write!(f, "%"),
            Token::Star => write!(f, "*"),
            Token::Ampersand => write!(f, "&"),
            Token::Semicolon => write!(f, ";"),
            Token::Backtick => write!(f, "`"),
            Token::Tilde => write!(f, "~"),
            Token::Caret => write!(f, "^"),
            Token::Backslash => write!(f, "\\"),
            Token::Underscore => write!(f, "_"),
            Token::Apostrophe => write!(f, "'"),
            Token::UnicodeText(s) => write!(f, "{}", s),
            Token::Ident(s) => write!(f, "{}", s),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::NumberLit(n) => write!(f, "{}", n),
            Token::Comment(s) => write!(f, "# {}", s),
            Token::Newline => write!(f, "\\n"),
            Token::Indent => write!(f, "INDENT"),
            Token::Dedent => write!(f, "DEDENT"),
        }
    }
}

/// Span type for tokens.
pub type Span = SimpleSpan<usize>;

/// A token with its span.
pub type Spanned<T> = (T, Span);

/// Create the lexer parser.
pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    let comment = just('#')
        .ignore_then(none_of('\n').repeated().to_slice())
        .map(Token::Comment);

    // String literals (double-quoted only - single quotes are apostrophes in text)
    let string_lit = just('"')
        .ignore_then(none_of('"').repeated().to_slice())
        .then_ignore(just('"'))
        .map(Token::StringLit);

    // Number literals
    let number = text::int(10)
        .then(just('.').then(text::digits(10)).or_not())
        .to_slice()
        .map(|s: &str| Token::NumberLit(s.parse().unwrap()));

    // Multi-character operators (must come before single char versions)
    let multi_char_ops = choice((
        just(":->").to(Token::ColonArrow),
        just(":|").to(Token::ColonPipe),
        just("->").to(Token::Arrow),
        just("...").to(Token::Ellipsis),
        just("==").to(Token::Eq),
        just("!=").to(Token::Ne),
        just("<=").to(Token::Le),
        just(">=").to(Token::Ge),
        just("{!").to(Token::ExclBrace),
        just("{{").to(Token::DoubleLBrace),
        just("}}").to(Token::DoubleBrace),
    ));

    // Single character operators and punctuation
    let single_char_ops = choice((
        just('<').to(Token::Lt),
        just('>').to(Token::Gt),
        just('=').to(Token::Assign),
        just('+').to(Token::Plus),
        just('-').to(Token::Minus),
        just(':').to(Token::Colon),
        just('.').to(Token::Dot),
        just(',').to(Token::Comma),
        just('@').to(Token::At),
        just('|').to(Token::Pipe),
        just('(').to(Token::LParen),
        just(')').to(Token::RParen),
        just('[').to(Token::LBracket),
        just(']').to(Token::RBracket),
        just('{').to(Token::LBrace),
        just('}').to(Token::RBrace),
    ));

    // Additional punctuation that appears in instruction content
    let text_punctuation = choice((
        just('/').to(Token::Slash),
        just('?').to(Token::Question),
        just('!').to(Token::Exclamation),
        just('$').to(Token::Dollar),
        just('%').to(Token::Percent),
        just('*').to(Token::Star),
        just('&').to(Token::Ampersand),
        just(';').to(Token::Semicolon),
        just('`').to(Token::Backtick),
        just('~').to(Token::Tilde),
        just('^').to(Token::Caret),
        just('\\').to(Token::Backslash),
        just('_').to(Token::Underscore),
        just('\'').to(Token::Apostrophe),
    ));

    // Unicode text - handles emojis and other non-ASCII characters
    // Captures sequences of non-ASCII characters (emojis, special symbols, etc.)
    let unicode_text = any()
        .filter(|c: &char| !c.is_ascii())
        .repeated()
        .at_least(1)
        .to_slice()
        .map(Token::UnicodeText);

    // Block keywords
    let block_keywords = choice((
        text::keyword("config").to(Token::Config),
        text::keyword("variables").to(Token::Variables),
        text::keyword("system").to(Token::System),
        text::keyword("start_agent").to(Token::StartAgent),
        text::keyword("topic").to(Token::Topic),
        text::keyword("actions").to(Token::Actions),
        text::keyword("inputs").to(Token::Inputs),
        text::keyword("outputs").to(Token::Outputs),
        text::keyword("target").to(Token::Target),
        text::keyword("reasoning").to(Token::Reasoning),
        text::keyword("instructions").to(Token::Instructions),
        text::keyword("before_reasoning").to(Token::BeforeReasoning),
        text::keyword("after_reasoning").to(Token::AfterReasoning),
        text::keyword("messages").to(Token::Messages),
    ));

    // More keywords
    let more_keywords = choice((
        text::keyword("welcome").to(Token::Welcome),
        text::keyword("error").to(Token::Error),
        text::keyword("connection").to(Token::Connection),
        text::keyword("connections").to(Token::Connections),
        text::keyword("knowledge").to(Token::Knowledge),
        text::keyword("language").to(Token::Language),
        text::keyword("mutable").to(Token::Mutable),
        text::keyword("linked").to(Token::Linked),
        text::keyword("description").to(Token::Description),
        text::keyword("source").to(Token::Source),
        text::keyword("label").to(Token::Label),
        text::keyword("is_required").to(Token::IsRequired),
        text::keyword("is_displayable").to(Token::IsDisplayable),
        text::keyword("is_used_by_planner").to(Token::IsUsedByPlanner),
        text::keyword("complex_data_type_name").to(Token::ComplexDataTypeName),
        text::keyword("filter_from_agent").to(Token::FilterFromAgent),
        text::keyword("require_user_confirmation").to(Token::RequireUserConfirmation),
        text::keyword("include_in_progress_indicator").to(Token::IncludeInProgressIndicator),
        text::keyword("progress_indicator_message").to(Token::ProgressIndicatorMessage),
    ));

    // Type keywords
    let type_keywords = choice((
        text::keyword("string").to(Token::String),
        text::keyword("number").to(Token::Number),
        text::keyword("boolean").to(Token::Boolean),
        text::keyword("object").to(Token::Object),
        text::keyword("list").to(Token::List),
        text::keyword("date").to(Token::Date),
        text::keyword("timestamp").to(Token::Timestamp),
        text::keyword("currency").to(Token::Currency),
        text::keyword("datetime").to(Token::Datetime),
        text::keyword("time").to(Token::Time),
        text::keyword("integer").to(Token::Integer),
        text::keyword("long").to(Token::Long),
        text::keyword("id").to(Token::Id),
    ));

    // Statement keywords
    let stmt_keywords = choice((
        text::keyword("if").to(Token::If),
        text::keyword("else").to(Token::Else),
        text::keyword("run").to(Token::Run),
        text::keyword("with").to(Token::With),
        text::keyword("set").to(Token::Set),
        text::keyword("to").to(Token::To),
        text::keyword("as").to(Token::As),
        text::keyword("transition").to(Token::Transition),
        text::keyword("available").to(Token::Available),
        text::keyword("when").to(Token::When),
    ));

    // Literal and operator keywords
    let lit_op_keywords = choice((
        text::keyword("True").to(Token::True),
        text::keyword("False").to(Token::False),
        text::keyword("None").to(Token::None),
        text::keyword("is").to(Token::Is),
        text::keyword("not").to(Token::Not),
        text::keyword("and").to(Token::And),
        text::keyword("or").to(Token::Or),
    ));

    // Combine all keywords
    let keyword =
        choice((block_keywords, more_keywords, type_keywords, stmt_keywords, lit_op_keywords));

    // Identifier: starts with letter or underscore, followed by alphanumeric or underscore
    let ident = text::ident().map(Token::Ident);

    // Newline
    let newline = just('\n').to(Token::Newline);

    // All tokens - combine in groups to stay under tuple size limits
    let token = choice((
        comment,
        string_lit,
        number,
        multi_char_ops,
        single_char_ops,
        text_punctuation,
        unicode_text,
        keyword,
        ident,
        newline,
    ));

    // Horizontal whitespace (spaces and tabs, but not newlines)
    let horizontal_ws = one_of(" \t").repeated();

    token
        .map_with(|tok, e| (tok, e.span()))
        .padded_by(horizontal_ws)
        .repeated()
        .collect()
}

/// Process raw tokens to add INDENT/DEDENT tokens based on indentation.
///
/// Uses Python-style dynamic indentation tracking:
/// - First indented line sets the indent level for that block
/// - INDENT is emitted when going to a deeper level
/// - DEDENT is emitted when returning to a shallower level
/// - Indent levels are tracked on a stack
pub fn add_indentation_tokens<'src>(
    source: &'src str,
    tokens: Vec<Spanned<Token<'src>>>,
) -> Vec<Spanned<Token<'src>>> {
    let mut result = Vec::with_capacity(tokens.len() * 2);
    let mut indent_stack: Vec<usize> = vec![0]; // Stack of indentation levels (Python-style)

    // Build a map of byte positions to their line indentation
    let line_indents: Vec<(usize, usize)> = source
        .lines()
        .scan(0usize, |pos, line| {
            let start = *pos;
            *pos += line.len() + 1; // +1 for newline
            let indent = line.len() - line.trim_start().len();
            Some((start, indent))
        })
        .collect();

    // Helper to find indentation at a given position
    let get_indent_at = |pos: usize| -> usize {
        for (line_start, indent) in line_indents.iter().rev() {
            if pos >= *line_start {
                return *indent;
            }
        }
        0
    };

    let mut i = 0;
    while i < tokens.len() {
        let (tok, span) = &tokens[i];

        if matches!(tok, Token::Newline) {
            result.push((tok.clone(), *span));

            // Look at next non-comment, non-newline token to determine indentation
            let mut next_idx = i + 1;
            while next_idx < tokens.len() {
                match &tokens[next_idx].0 {
                    Token::Comment(_) => {
                        // Push comments to result, continue looking
                        result.push(tokens[next_idx].clone());
                        next_idx += 1;
                    }
                    Token::Newline => {
                        // Skip blank lines
                        result.push(tokens[next_idx].clone());
                        next_idx += 1;
                    }
                    _ => break,
                }
            }

            if next_idx < tokens.len() {
                let next_span = &tokens[next_idx].1;
                let new_indent = get_indent_at(next_span.start);
                let current_indent = *indent_stack.last().unwrap_or(&0);

                if new_indent > current_indent {
                    // Python-style: push the actual new indent level onto the stack
                    // This handles any indent size (2, 3, 4 spaces, etc.)
                    indent_stack.push(new_indent);
                    result.push((Token::Indent, Span::new((), next_span.start..next_span.start)));
                } else if new_indent < current_indent {
                    // Python-style: pop from stack until we find a matching level
                    // Emit DEDENT for each level we pop
                    while indent_stack.len() > 1 && *indent_stack.last().unwrap() > new_indent {
                        indent_stack.pop();
                        result.push((Token::Dedent, Span::new((), next_span.start..next_span.start)));
                    }
                    // Note: In strict Python, if new_indent doesn't match any stack level,
                    // it's an IndentationError. We're lenient here and just dedent to nearest.
                }
                // If new_indent == current_indent, no change - same level
            }
            i = next_idx;
        } else {
            result.push((tok.clone(), *span));
            i += 1;
        }
    }

    // Emit remaining DEDENTs at EOF
    let eof_pos = source.len();
    while indent_stack.len() > 1 {
        indent_stack.pop();
        result.push((Token::Dedent, Span::new((), eof_pos..eof_pos)));
    }

    result
}

/// Full lexer that produces indentation-aware tokens.
pub fn lex_with_indentation<'src>(
    source: &'src str,
) -> Result<Vec<Spanned<Token<'src>>>, Vec<Rich<'src, char, Span>>> {
    let tokens = lexer().parse(source).into_result()?;
    Ok(add_indentation_tokens(source, tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let input = "config: agent_name";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::Config, Token::Colon, Token::Ident("agent_name"),]);
    }

    #[test]
    fn test_string_literal() {
        let input = r#""hello world""#;
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::StringLit("hello world")]);
    }

    #[test]
    fn test_reference_tokens() {
        let input = "@variables.user_id";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::At,
                Token::Variables,
                Token::Dot,
                Token::Ident("user_id"),
            ]
        );
    }

    #[test]
    fn test_operators() {
        let input = "== != < > <= >= = + -";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Eq,
                Token::Ne,
                Token::Lt,
                Token::Gt,
                Token::Le,
                Token::Ge,
                Token::Assign,
                Token::Plus,
                Token::Minus,
            ]
        );
    }

    #[test]
    fn test_ellipsis() {
        let input = "with value=...";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::With,
                Token::Ident("value"),
                Token::Assign,
                Token::Ellipsis
            ]
        );
    }

    #[test]
    fn test_colon_variants() {
        let input = ": :| :->";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::Colon, Token::ColonPipe, Token::ColonArrow]);
    }

    #[test]
    fn test_number_literals() {
        let input = "42 3.14 0";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::NumberLit(42.0),
                Token::NumberLit(3.14),
                Token::NumberLit(0.0),
            ]
        );
    }

    #[test]
    fn test_interpolation_brace() {
        let input = "{!@variables.name}";
        let result = lexer().parse(input).into_result();
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::ExclBrace,
                Token::At,
                Token::Variables,
                Token::Dot,
                Token::Ident("name"),
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn test_indentation_tokens() {
        let input = r#"config:
   agent_name: "Test"
   description: "Desc"

topic main:
   description: "Main"
"#;
        let result = lex_with_indentation(input);
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();

        // Should have INDENT after "config:" newline, DEDENT before "topic"
        assert!(tokens.contains(&Token::Indent));
        assert!(tokens.contains(&Token::Dedent));

        // Count indents and dedents
        let indents = tokens.iter().filter(|t| matches!(t, Token::Indent)).count();
        let dedents = tokens.iter().filter(|t| matches!(t, Token::Dedent)).count();

        // Should balance
        assert_eq!(indents, dedents, "INDENT/DEDENT should balance");
    }

    #[test]
    fn test_nested_indentation() {
        let input = r#"topic main:
   reasoning:
      instructions: "test"
"#;
        let result = lex_with_indentation(input);
        assert!(result.is_ok());
        let tokens: Vec<_> = result.unwrap().into_iter().map(|(t, _)| t).collect();

        // Should have 2 INDENTs and 2 DEDENTs
        let indents = tokens.iter().filter(|t| matches!(t, Token::Indent)).count();
        let dedents = tokens.iter().filter(|t| matches!(t, Token::Dedent)).count();
        assert_eq!(indents, 2, "Should have 2 INDENTs");
        assert_eq!(dedents, 2, "Should have 2 DEDENTs");
    }
}
