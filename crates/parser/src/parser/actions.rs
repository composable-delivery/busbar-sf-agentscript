//! Actions block parser.
//!
//! Parses action definitions with inputs, outputs, and targets.

use crate::ast::{ActionDef, ActionsBlock, ParamDef, Spanned};
use crate::lexer::Token;
use chumsky::prelude::*;

use super::expressions::spanned_type;
use super::primitives::{
    dedent, description_entry, indent, newline, skip_block_noise, spanned_ident, spanned_string,
    to_ast_span, ParserInput, Span,
};

/// Helper enum for param def entries.
#[derive(Clone)]
enum ParamDefEntry {
    Description(Spanned<String>),
    Label(Spanned<String>),
    IsRequired(Spanned<bool>),
    IsDisplayable(Spanned<bool>),
    IsUsedByPlanner,
    ComplexDataTypeName(Spanned<String>),
    FilterFromAgent(Spanned<bool>),
}

/// Parse a parameter definition (for inputs/outputs).
pub(crate) fn param_def<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ParamDef>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    // Parameter name can be identifier, quoted string, or certain keywords used as param names
    let param_name = choice((
        spanned_ident(),
        spanned_string(),
        just(Token::Description)
            .map_with(|_, e| Spanned::new("description".to_string(), to_ast_span(e.span()))),
        just(Token::Available)
            .map_with(|_, e| Spanned::new("available".to_string(), to_ast_span(e.span()))),
    ));

    let param_entry = choice((
        description_entry().map(ParamDefEntry::Description),
        just(Token::Label)
            .ignore_then(just(Token::Colon))
            .ignore_then(spanned_string())
            .map(ParamDefEntry::Label),
        just(Token::IsRequired)
            .ignore_then(just(Token::Colon))
            .ignore_then(choice((just(Token::True).to(true), just(Token::False).to(false))))
            .map_with(|v, e| ParamDefEntry::IsRequired(Spanned::new(v, to_ast_span(e.span())))),
        just(Token::IsDisplayable)
            .ignore_then(just(Token::Colon))
            .ignore_then(choice((just(Token::True).to(true), just(Token::False).to(false))))
            .map_with(|v, e| ParamDefEntry::IsDisplayable(Spanned::new(v, to_ast_span(e.span())))),
        just(Token::IsUsedByPlanner)
            .ignore_then(just(Token::Colon))
            .ignore_then(choice((just(Token::True).to(true), just(Token::False).to(false))))
            .to(ParamDefEntry::IsUsedByPlanner),
        just(Token::ComplexDataTypeName)
            .ignore_then(just(Token::Colon))
            .ignore_then(spanned_string())
            .map(ParamDefEntry::ComplexDataTypeName),
        just(Token::FilterFromAgent)
            .ignore_then(just(Token::Colon))
            .ignore_then(choice((just(Token::True).to(true), just(Token::False).to(false))))
            .map_with(|v, e| ParamDefEntry::FilterFromAgent(Spanned::new(v, to_ast_span(e.span())))),
    ));

    param_name
        .then_ignore(just(Token::Colon))
        .then(spanned_type())
        .then(
            newline()
                .ignore_then(skip_block_noise())
                .ignore_then(indent())
                .ignore_then(
                    param_entry
                        .separated_by(skip_block_noise())
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(skip_block_noise())
                .then_ignore(dedent())
                .or_not()
                .map(|opt| opt.unwrap_or_default()),
        )
        .map_with(|((name, ty), entries), e| {
            let mut description = None;
            let mut label = None;
            let mut is_required = None;
            let mut is_displayable = None;
            let mut complex_data_type_name = None;
            let mut filter_from_agent = None;

            for entry in entries {
                match entry {
                    ParamDefEntry::Description(d) => description = Some(d),
                    ParamDefEntry::Label(l) => label = Some(l),
                    ParamDefEntry::IsRequired(v) => is_required = Some(v),
                    ParamDefEntry::IsDisplayable(v) => is_displayable = Some(v),
                    ParamDefEntry::IsUsedByPlanner => {
                        // Parsed for compatibility but not stored
                    }
                    ParamDefEntry::ComplexDataTypeName(s) => complex_data_type_name = Some(s),
                    ParamDefEntry::FilterFromAgent(v) => filter_from_agent = Some(v),
                }
            }

            Spanned::new(
                ParamDef {
                    name,
                    ty,
                    description,
                    label,
                    is_required,
                    filter_from_agent,
                    is_displayable,
                    complex_data_type_name,
                },
                to_ast_span(e.span()),
            )
        })
}

/// Helper enum for action definition entries.
#[derive(Clone)]
enum ActionDefEntry {
    Description(Spanned<String>),
    Label(Spanned<String>),
    RequireUserConfirmation(Spanned<bool>),
    IncludeInProgressIndicator(Spanned<bool>),
    ProgressIndicatorMessage(Spanned<String>),
    Target(Spanned<String>),
    Inputs(Vec<Spanned<ParamDef>>),
    Outputs(Vec<Spanned<ParamDef>>),
}

/// Helper enum for inputs/outputs block entries (can be param or description).
#[derive(Clone)]
enum InputOutputEntry {
    Param(Box<Spanned<ParamDef>>),
    Description,
}

/// Parse an entry inside inputs/outputs block (either param_def or description).
fn input_output_entry<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    InputOutputEntry,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    choice((
        // Try description first since it's simpler
        description_entry().to(InputOutputEntry::Description),
        // Then try param_def
        param_def().map(|p| InputOutputEntry::Param(Box::new(p))),
    ))
}

/// Extract params from input/output entries, ignoring descriptions.
fn extract_params(entries: Vec<InputOutputEntry>) -> Vec<Spanned<ParamDef>> {
    entries
        .into_iter()
        .filter_map(|e| match e {
            InputOutputEntry::Param(p) => Some(*p),
            InputOutputEntry::Description => None, // Block-level description, ignored for now
        })
        .collect()
}

/// Parse an action definition.
pub(crate) fn action_def<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ActionDef>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    spanned_ident()
        .then_ignore(just(Token::Colon))
        .then_ignore(newline())
        .then_ignore(skip_block_noise())
        .then_ignore(indent())
        .labelled("action definition")
        .then(
            choice((
                description_entry().map(ActionDefEntry::Description),
                just(Token::Label)
                    .ignore_then(just(Token::Colon))
                    .ignore_then(spanned_string())
                    .map(ActionDefEntry::Label),
                just(Token::RequireUserConfirmation)
                    .ignore_then(just(Token::Colon))
                    .ignore_then(choice((just(Token::True).to(true), just(Token::False).to(false))))
                    .map_with(|v, e| ActionDefEntry::RequireUserConfirmation(Spanned::new(v, to_ast_span(e.span())))),
                just(Token::IncludeInProgressIndicator)
                    .ignore_then(just(Token::Colon))
                    .ignore_then(choice((just(Token::True).to(true), just(Token::False).to(false))))
                    .map_with(|v, e| ActionDefEntry::IncludeInProgressIndicator(Spanned::new(v, to_ast_span(e.span())))),
                just(Token::ProgressIndicatorMessage)
                    .ignore_then(just(Token::Colon))
                    .ignore_then(spanned_string())
                    .map(ActionDefEntry::ProgressIndicatorMessage),
                just(Token::Target)
                    .ignore_then(just(Token::Colon))
                    .ignore_then(spanned_string())
                    .map(ActionDefEntry::Target),
                just(Token::Inputs)
                    .map_with(|_, e| e.span()) // Capture the span of 'inputs'
                    .then_ignore(just(Token::Colon))
                    .then_ignore(newline())
                    .then_ignore(skip_block_noise())
                    .then(
                        // Check if next token is DEDENT (empty block) without consuming it
                        just(Token::Dedent)
                            .rewind()
                            .to(None) // Empty block detected
                            .or(
                                indent()
                                    .ignore_then(
                                        input_output_entry()
                                            .separated_by(skip_block_noise())
                                            .allow_trailing()
                                            .collect::<Vec<_>>(),
                                    )
                                    .then_ignore(skip_block_noise())
                                    .then_ignore(dedent())
                                    .map(Some)
                            )
                    )
                    .validate(|(inputs_span, entries), _, emitter| {
                        if entries.is_none() {
                            emitter.emit(Rich::custom(inputs_span, "inputs block cannot be empty"));
                        }
                        entries.unwrap_or_default()
                    })
                    .labelled("inputs block")
                    .map(|entries| ActionDefEntry::Inputs(extract_params(entries))),
                just(Token::Outputs)
                    .map_with(|_, e| e.span()) // Capture the span of 'outputs'
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
                                        input_output_entry()
                                            .separated_by(skip_block_noise())
                                            .allow_trailing()
                                            .collect::<Vec<_>>(),
                                    )
                                    .then_ignore(skip_block_noise())
                                    .then_ignore(dedent())
                                    .map(Some)
                            )
                    )
                    .validate(|(outputs_span, entries), _, emitter| {
                        if entries.is_none() {
                            emitter.emit(Rich::custom(outputs_span, "outputs block cannot be empty"));
                        }
                        entries.unwrap_or_default()
                    })
                    .labelled("outputs block")
                    .map(|entries| ActionDefEntry::Outputs(extract_params(entries))),
            ))
            .separated_by(skip_block_noise())
            .allow_trailing()
            .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .map_with(|(name, entries), e| {
            let mut def = ActionDef {
                name,
                description: None,
                label: None,
                require_user_confirmation: None,
                include_in_progress_indicator: None,
                progress_indicator_message: None,
                inputs: None,
                outputs: None,
                target: None,
            };

            for entry in entries {
                match entry {
                    ActionDefEntry::Description(d) => def.description = Some(d),
                    ActionDefEntry::Label(l) => def.label = Some(l),
                    ActionDefEntry::RequireUserConfirmation(v) => def.require_user_confirmation = Some(v),
                    ActionDefEntry::IncludeInProgressIndicator(v) => def.include_in_progress_indicator = Some(v),
                    ActionDefEntry::ProgressIndicatorMessage(m) => def.progress_indicator_message = Some(m),
                    ActionDefEntry::Target(t) => def.target = Some(t),
                    ActionDefEntry::Inputs(i) => {
                        def.inputs = Some(Spanned::new(i, to_ast_span(e.span())))
                    }
                    ActionDefEntry::Outputs(o) => {
                        def.outputs = Some(Spanned::new(o, to_ast_span(e.span())))
                    }
                }
            }

            Spanned::new(def, to_ast_span(e.span()))
        })
}

/// Parse the actions block.
pub(crate) fn actions_block<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<ActionsBlock>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    just(Token::Actions)
        .ignore_then(just(Token::Colon))
        .ignore_then(newline())
        .ignore_then(skip_block_noise())
        .ignore_then(indent())
        .ignore_then(
            action_def()
                .separated_by(skip_block_noise())
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_block_noise())
        .then_ignore(dedent())
        .labelled("actions block")
        .map_with(|actions, e| Spanned::new(ActionsBlock { actions }, to_ast_span(e.span())))
}
