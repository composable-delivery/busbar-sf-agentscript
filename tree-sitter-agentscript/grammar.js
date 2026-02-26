/**
 * Tree-sitter grammar for Salesforce AgentScript (.agent files)
 *
 * AgentScript is an indentation-sensitive language for defining AI agent behavior.
 * Uses 3-space indentation (like YAML but stricter).
 */

module.exports = grammar({
  name: 'agentscript',

  // External scanner handles INDENT/DEDENT/NEWLINE and interpolation tokens
  externals: $ => [
    $._newline,
    $._indent,
    $._dedent,
    $.interpolation_start,  // {! - must be scanned as a unit
    $.instruction_text_segment,  // Text that doesn't contain {! or newline
  ],

  // Token precedence (identity is between and and comparison per AST)
  precedences: $ => [
    ['ternary', 'or', 'and', 'identity', 'comparison', 'additive', 'unary', 'postfix', 'primary'],
  ],

  // Extra tokens (ignored between rules)
  extras: $ => [
    /[ \t]/,
    $.comment,
  ],

  // Word token for keyword extraction
  word: $ => $.identifier,

  // Conflict resolution
  conflicts: $ => [
    [$.reference, $.expression],
    [$.with_clause, $.ternary_expression],
    [$.available_when_clause, $.ternary_expression],
    [$.set_clause, $.ternary_expression],
    [$.set_statement, $.ternary_expression],
    [$.set_statement, $.binary_expression],
    [$.instruction_text_with_interpolation],
  ],

  rules: {
    // =========================================================================
    // Source File - Top Level
    // =========================================================================
    source_file: $ => repeat(choice(
      $.config_block,
      $.variables_block,
      $.system_block,
      $.start_agent_block,
      $.topic_block,
      $.connection_block,
      $.language_block,
      $.knowledge_block,
      $._newline,
    )),

    // =========================================================================
    // Config Block
    // =========================================================================
    config_block: $ => seq(
      'config',
      ':',
      $._indent,
      repeat(choice($.config_entry, $._newline)),
      $._dedent,
    ),

    config_entry: $ => choice(
      seq('agent_name', ':', $.string),
      seq('agent_label', ':', $.string),
      seq('description', ':', $.string),
      seq('agent_type', ':', $.string),
      seq('default_agent_user', ':', $.string),
    ),

    // =========================================================================
    // Variables Block
    // =========================================================================
    variables_block: $ => seq(
      'variables',
      ':',
      $._indent,
      repeat(choice($.variable_def, $._newline)),
      $._dedent,
    ),

    variable_def: $ => seq(
      $.identifier,
      ':',
      $.variable_modifier,
      $.type,
      optional(seq('=', $.expression)),
      optional(seq(
        $._indent,
        repeat(choice($.variable_entry, $._newline)),
        $._dedent,
      )),
    ),

    variable_modifier: $ => choice('mutable', 'linked'),

    variable_entry: $ => choice(
      $.description_entry,
      seq('source', ':', choice($.string, $.reference)),
      seq('label', ':', $.string),
    ),

    // =========================================================================
    // System Block
    // =========================================================================
    system_block: $ => seq(
      'system',
      ':',
      $._indent,
      repeat(choice($.system_entry, $._newline)),
      $._dedent,
    ),

    system_entry: $ => choice(
      $.instructions_block,
      $.messages_block,
    ),

    messages_block: $ => seq(
      'messages',
      ':',
      $._indent,
      repeat(choice($.message_entry, $._newline)),
      $._dedent,
    ),

    message_entry: $ => choice(
      seq('welcome', ':', $.string),
      seq('error', ':', $.string),
    ),

    // =========================================================================
    // Start Agent Block
    // =========================================================================
    start_agent_block: $ => seq(
      'start_agent',
      $.identifier,
      ':',
      $._indent,
      repeat(choice($.start_agent_entry, $._newline)),
      $._dedent,
    ),

    start_agent_entry: $ => choice(
      $.description_entry,
      $.system_block,
      $.actions_block,
      $.reasoning_block,
      $.before_reasoning_block,
      $.after_reasoning_block,
    ),

    // =========================================================================
    // Topic Block
    // =========================================================================
    topic_block: $ => seq(
      'topic',
      $.identifier,
      ':',
      $._indent,
      repeat(choice($.topic_entry, $._newline)),
      $._dedent,
    ),

    topic_entry: $ => choice(
      $.description_entry,
      $.system_block,
      $.reasoning_block,
      $.before_reasoning_block,
      $.after_reasoning_block,
      $.actions_block,
    ),

    // =========================================================================
    // Connection Block
    // =========================================================================
    connection_block: $ => seq(
      'connection',
      $.identifier,
      ':',
      $._indent,
      repeat(choice($.connection_entry, $._newline)),
      $._dedent,
    ),

    connection_entry: $ => choice(
      $.description_entry,
      seq('target', ':', $.string),
      seq('escalation_message', ':', $.string),
      seq('outbound_route_type', ':', $.string),
      seq('outbound_route_name', ':', $.string),
    ),

    // =========================================================================
    // Language Block
    // =========================================================================
    language_block: $ => seq(
      'language',
      ':',
      $._indent,
      repeat(choice($.language_entry, $._newline)),
      $._dedent,
    ),

    language_entry: $ => choice(
      seq('locale', ':', $.string),
      seq('default_locale', ':', $.string),
      seq('additional_locales', ':', $.string),
    ),

    // =========================================================================
    // Knowledge Block
    // =========================================================================
    knowledge_block: $ => seq(
      'knowledge',
      ':',
      $._indent,
      repeat(choice($.knowledge_entry, $._newline)),
      $._dedent,
    ),

    knowledge_entry: $ => choice(
      $.description_entry,
      seq('source', ':', $.string),
    ),

    // =========================================================================
    // Actions Block (inside topics)
    // =========================================================================
    actions_block: $ => seq(
      'actions',
      ':',
      $._indent,
      repeat(choice($.action_def, $._newline)),
      $._dedent,
    ),

    action_def: $ => seq(
      $.identifier,
      ':',
      $._indent,
      repeat(choice($.action_entry, $._newline)),
      $._dedent,
    ),

    action_entry: $ => choice(
      $.description_entry,
      seq('label', ':', $.string),
      seq('target', ':', $.string),
      seq('require_user_confirmation', ':', $.boolean),
      seq('include_in_progress_indicator', ':', $.boolean),
      seq('progress_indicator_message', ':', $.string),
      $.inputs_block,
      $.outputs_block,
    ),

    inputs_block: $ => prec.left(seq(
      'inputs',
      ':',
      $._indent,
      repeat(choice($.param_def, $._newline)),
      $._dedent,
    )),

    outputs_block: $ => prec.left(seq(
      'outputs',
      ':',
      $._indent,
      repeat(choice($.param_def, $._newline)),
      $._dedent,
    )),

    param_def: $ => seq(
      field('name', choice($.identifier, $.string)),
      ':',
      field('type', $.type),
      optional(seq(
        $._indent,
        repeat(choice($.param_entry, $._newline)),
        $._dedent,
      )),
    ),

    param_entry: $ => choice(
      $.description_entry,
      seq('label', ':', $.string),
      seq('is_required', ':', $.boolean),
      seq('is_displayable', ':', $.boolean),
      seq('is_used_by_planner', ':', $.boolean),
      seq('filter_from_agent', ':', $.boolean),
      seq('complex_data_type_name', ':', $.string),
    ),

    // =========================================================================
    // Reasoning Block
    // =========================================================================
    reasoning_block: $ => seq(
      'reasoning',
      ':',
      $._indent,
      repeat(choice($.reasoning_entry, $._newline)),
      $._dedent,
    ),

    reasoning_entry: $ => choice(
      $.instructions_block,
      $.reasoning_actions_block,
    ),

    before_reasoning_block: $ => seq(
      'before_reasoning',
      ':',
      $._indent,
      repeat(choice($.directive_statement, $._newline)),
      $._dedent,
    ),

    after_reasoning_block: $ => seq(
      'after_reasoning',
      ':',
      $._indent,
      repeat(choice($.directive_statement, $._newline)),
      $._dedent,
    ),

    // =========================================================================
    // Instructions
    // =========================================================================
    instructions_block: $ => choice(
      // Simple: instructions: "..."
      seq('instructions', ':', $.string),
      // Pipe-prefixed multiline text: instructions:|
      seq(
        'instructions',
        ':',
        '|',
        $._indent,
        repeat(choice($.plain_instruction_line, $._newline)),
        $._dedent,
      ),
      // Dynamic instructions: instructions:->
      seq(
        'instructions',
        ':',
        '->',
        $._indent,
        repeat(choice($.dynamic_instruction, $._newline)),
        $._dedent,
      ),
      // Static block (legacy)
      seq(
        'instructions',
        ':',
        $._indent,
        repeat(choice($.instruction_line, $._newline)),
        $._dedent,
      ),
    ),

    plain_instruction_line: $ => $.instruction_text,

    instruction_line: $ => choice(
      $.string,
      seq('|', $.instruction_text),
      seq('-', $.instruction_text),
    ),

    // Instruction line with interpolation support for dynamic instructions
    // Can have continuation lines that are more indented
    dynamic_instruction_line: $ => choice(
      $.string,
      seq('|', $.instruction_text_with_interpolation, optional($.instruction_continuation)),
      seq('-', $.instruction_text_with_interpolation, optional($.instruction_continuation)),
    ),

    // Continuation lines for multi-line instruction text
    // These are lines that are more indented than the | or - line
    instruction_continuation: $ => seq(
      $._indent,
      repeat1(choice($.instruction_continuation_line, $._newline)),
      $._dedent,
    ),

    // A single continuation line (text with optional interpolation)
    instruction_continuation_line: $ => $.instruction_text_with_interpolation,

    dynamic_instruction: $ => choice(
      $.dynamic_instruction_line,
      $.if_instruction,
      $.run_statement,
      $.set_statement,
      $.transition_instruction,
    ),

    // Transition can appear as a statement in dynamic instructions
    transition_instruction: $ => seq(
      $.utils_transition,
      'to',
      $.reference,
    ),

    if_instruction: $ => seq(
      'if',
      $.expression,
      ':',
      $._indent,
      repeat(choice($.dynamic_instruction, $._newline)),
      $._dedent,
      optional($.else_instruction),
    ),

    else_instruction: $ => seq(
      'else',
      ':',
      $._indent,
      repeat(choice($.dynamic_instruction, $._newline)),
      $._dedent,
    ),

    // Plain instruction text without interpolation parsing
    instruction_text: $ => /[^\n]+/,

    // Interpolation syntax: {!expression}
    // interpolation_start is handled by external scanner to properly recognize {!
    interpolation: $ => seq(
      $.interpolation_start,
      $.expression,
      '}',
    ),

    // Instruction text that may contain interpolations
    // instruction_text_segment is handled by external scanner
    instruction_text_with_interpolation: $ => repeat1(choice(
      $.interpolation,
      $.instruction_text_segment,
    )),

    // =========================================================================
    // Reasoning Actions
    // =========================================================================
    reasoning_actions_block: $ => seq(
      'actions',
      ':',
      $._indent,
      repeat(choice($.reasoning_action, $._newline)),
      $._dedent,
    ),

    reasoning_action: $ => seq(
      field('name', $.identifier),
      ':',
      field('target', $.reasoning_target),
      optional(seq(
        $._indent,
        repeat(choice($.reasoning_action_entry, $._newline)),
        $._dedent,
      )),
    ),

    reasoning_target: $ => choice(
      // @utils.transition to @topic.name - must come before plain reference
      seq($.utils_transition, 'to', $.reference),
      // @utils.escalate
      $.utils_escalate,
      // @utils.setVariables
      $.utils_set_variables,
      // @topic.name - topic delegation (must come before plain reference)
      $.topic_reference,
      // Plain reference (e.g., @actions.something)
      $.reference,
    ),

    // Token-level patterns for utils functions
    utils_transition: $ => token('@utils.transition'),
    utils_escalate: $ => token('@utils.escalate'),
    utils_set_variables: $ => token('@utils.setVariables'),

    // Topic reference for topic delegation: @topic.name
    topic_reference: $ => token(seq('@topic.', /[a-zA-Z_][a-zA-Z0-9_]*/)),

    reasoning_action_entry: $ => choice(
      $.description_entry,
      $.with_clause,
      $.set_clause,
      $.run_clause,
      $.available_when_clause,
      $.if_clause,
      $.transition_clause,
    ),

    with_clause: $ => seq(
      'with',
      field('param', $.identifier),
      '=',
      field('value', $.expression),
    ),

    set_clause: $ => seq(
      'set',
      field('target', $.reference),
      '=',
      field('value', $.expression),
    ),

    run_clause: $ => seq(
      'run',
      field('action', $.reference),
      optional(seq(
        $._indent,
        repeat(choice($.with_clause, $.set_clause, $._newline)),
        $._dedent,
      )),
    ),

    available_when_clause: $ => prec.right(10, seq(
      'available',
      'when',
      $.expression,
    )),

    if_clause: $ => seq(
      'if',
      field('condition', $.expression),
      ':',
      $._indent,
      optional($.transition_clause),
      $._dedent,
    ),

    transition_clause: $ => seq(
      'transition',
      'to',
      $.reference,
    ),

    // =========================================================================
    // Directive Statements (for before/after_reasoning)
    // =========================================================================
    directive_statement: $ => choice(
      $.run_statement,
      $.set_statement,
      $.if_statement,
    ),

    set_statement: $ => seq(
      'set',
      field('target', $.reference),
      '=',
      field('value', $.expression),
    ),

    run_statement: $ => seq(
      'run',
      $.reference,
      optional(seq(
        $._indent,
        repeat(choice($.with_clause, $.set_clause, $._newline)),
        $._dedent,
      )),
    ),

    if_statement: $ => seq(
      'if',
      $.expression,
      ':',
      $._indent,
      repeat(choice($.directive_statement, $._newline)),
      $._dedent,
      optional($.else_clause),
    ),

    else_clause: $ => seq(
      'else',
      ':',
      $._indent,
      repeat(choice($.directive_statement, $._newline)),
      $._dedent,
    ),

    // =========================================================================
    // Common Entries
    // =========================================================================
    description_entry: $ => seq('description', ':', $.string),

    // =========================================================================
    // Types
    // =========================================================================
    type: $ => choice(
      'string',
      'number',
      'boolean',
      'object',
      'integer',
      'long',
      'date',
      'datetime',
      'time',
      'timestamp',
      'currency',
      'id',
      $.list_type,
    ),

    list_type: $ => seq('list', '[', $.type, ']'),

    // =========================================================================
    // Expressions
    // =========================================================================
    expression: $ => choice(
      $.primary_expression,
      $.unary_expression,
      $.binary_expression,
      $.ternary_expression,
    ),

    primary_expression: $ => choice(
      $.string,
      $.number,
      $.boolean,
      $.none,
      $.reference,
      $.list,
      $.parenthesized_expression,
      $.property_access,
      $.index_access,
    ),

    parenthesized_expression: $ => seq('(', $.expression, ')'),

    unary_expression: $ => prec.left('unary', choice(
      seq('not', $.expression),
      seq('-', $.expression),
    )),

    // Binary expressions with explicit numeric precedence (higher = tighter binding)
    binary_expression: $ => choice(
      prec.left(1, seq($.expression, 'or', $.expression)),
      prec.left(2, seq($.expression, 'and', $.expression)),
      prec.left(3, seq($.expression, $.identity_operator, $.expression)),
      prec.left(4, seq($.expression, $.comparison_operator, $.expression)),
      prec.left(5, seq($.expression, choice('+', '-'), $.expression)),
    ),

    // Identity operators for None checks (is, is not)
    // Use prec to ensure "is not" is preferred over "is" followed by unary "not"
    identity_operator: $ => choice(
      prec(2, seq('is', 'not')),  // "is not" as compound operator (higher precedence)
      prec(1, 'is'),               // "is" alone (lower precedence)
    ),

    comparison_operator: $ => choice('==', '!=', '<', '>', '<=', '>='),

    ternary_expression: $ => prec.right('ternary', seq(
      field('then', $.expression),
      'if',
      field('condition', $.expression),
      'else',
      field('else', $.expression),
    )),

    property_access: $ => prec.left('postfix', seq(
      $.expression,
      '.',
      $.identifier,
    )),

    index_access: $ => prec.left('postfix', seq(
      $.expression,
      '[',
      $.expression,
      ']',
    )),

    list: $ => seq(
      '[',
      optional(seq(
        $.expression,
        repeat(seq(',', $.expression)),
        optional(','),
      )),
      ']',
    ),

    // =========================================================================
    // References
    // =========================================================================
    // Reference is a token-level pattern: @identifier.identifier.identifier...
    // This prevents property_access from splitting it
    reference: $ => token(seq(
      '@',
      /[a-zA-Z_][a-zA-Z0-9_]*/,
      repeat(seq('.', /[a-zA-Z_][a-zA-Z0-9_]*/)),
    )),

    // =========================================================================
    // Literals
    // =========================================================================
    string: $ => choice(
      $._double_quoted_string,
      $._single_quoted_string,
      $._triple_quoted_string,
    ),

    _double_quoted_string: $ => seq(
      '"',
      repeat(choice(
        /[^"\\]+/,
        $.escape_sequence,
      )),
      '"',
    ),

    _single_quoted_string: $ => seq(
      "'",
      repeat(choice(
        /[^'\\]+/,
        $.escape_sequence,
      )),
      "'",
    ),

    _triple_quoted_string: $ => seq(
      '"""',
      repeat(choice(
        /[^"]+/,
        /"[^"]/,
        /""[^"]/,
      )),
      '"""',
    ),

    escape_sequence: $ => /\\[\\'"nrt]/,

    number: $ => /\d+(\.\d+)?/,

    boolean: $ => choice('True', 'False'),

    none: $ => 'None',

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    comment: $ => /#[^\n]*/,
  },
});
