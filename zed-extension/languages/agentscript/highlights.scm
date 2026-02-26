; Syntax highlighting for AgentScript
; Used by editors like Neovim, Helix, VS Code (with tree-sitter extension)

; ============================================================================
; Comments
; ============================================================================
(comment) @comment

; ============================================================================
; Keywords - Block Types
; ============================================================================
[
  "config"
  "variables"
  "system"
  "start_agent"
  "topic"
  "connection"
  "language"
  "knowledge"
  "actions"
  "inputs"
  "outputs"
  "reasoning"
  "before_reasoning"
  "after_reasoning"
  "messages"
  "instructions"
] @keyword

; ============================================================================
; Keywords - Control Flow
; ============================================================================
[
  "if"
  "else"
  "transition"
  "to"
] @keyword.control

; ============================================================================
; Keywords - Actions
; ============================================================================
[
  "run"
  "set"
  "with"
  "available"
  "when"
] @keyword.function

; ============================================================================
; Keywords - Variable Modifiers
; ============================================================================
[
  "mutable"
  "linked"
] @keyword.modifier

; ============================================================================
; Property Names (keys)
; ============================================================================
[
  "agent_name"
  "agent_label"
  "agent_type"
  "default_agent_user"
  "description"
  "label"
  "source"
  "target"
  "locale"
  "welcome"
  "error"
  "is_required"
  "is_displayable"
  "is_used_by_planner"
  "filter_from_agent"
  "complex_data_type_name"
  "require_user_confirmation"
  "include_in_progress_indicator"
  "progress_indicator_message"
] @property

; ============================================================================
; Types
; ============================================================================
(type) @type
(list_type "list" @type.builtin)

[
  "string"
  "number"
  "boolean"
  "object"
  "integer"
  "long"
  "date"
  "datetime"
  "time"
  "timestamp"
  "currency"
  "id"
] @type.builtin

; ============================================================================
; Operators
; ============================================================================
[
  "+"
  "-"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "="
] @operator

[
  "and"
  "or"
  "not"
  "is"
] @keyword.operator

; Identity operators (is, is not)
(identity_operator) @keyword.operator

; ============================================================================
; Punctuation
; ============================================================================
[
  ":"
  "."
  ","
] @punctuation.delimiter

[
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

; ============================================================================
; Literals
; ============================================================================
(string) @string
(escape_sequence) @string.escape
(number) @number
(boolean) @boolean
(none) @constant.builtin

; ============================================================================
; References
; ============================================================================
; Reference is now a single token (e.g., @variables.name)
(reference) @variable.builtin

; ============================================================================
; Identifiers in Different Contexts
; ============================================================================

; Block names (topic name, connection name, etc.)
(topic_block
  "topic" @keyword
  (identifier) @label)

(start_agent_block
  "start_agent" @keyword
  (identifier) @label)

(connection_block
  "connection" @keyword
  (identifier) @label)

; Action definitions
(action_def
  (identifier) @function)

(reasoning_action
  name: (identifier) @function)

; Variable definitions
(variable_def
  (identifier) @variable)

; Parameter definitions
(param_def
  name: (identifier) @parameter)
(param_def
  name: (string) @parameter)

; With clause parameters
(with_clause
  param: (identifier) @parameter)

; Set clause targets
(set_clause
  target: (reference) @variable)

; Run clause actions
(run_clause
  action: (reference) @function.call)

(run_statement
  (reference) @function.call)

; ============================================================================
; Special Patterns - Reasoning Targets
; ============================================================================
; Note: @utils.transition, @utils.escalate, @utils.setVariables
; These are now parsed as tokens, so we match the whole reference
(reasoning_target
  (reference) @function.builtin)

; Utils tokens (built-in utility functions)
(utils_transition) @function.builtin
(utils_escalate) @function.builtin
(utils_set_variables) @function.builtin

; Topic reference (topic delegation)
(topic_reference) @variable.builtin

; ============================================================================
; Interpolation
; ============================================================================
; Interpolation delimiters in dynamic instructions
(interpolation_start) @punctuation.special
(interpolation "}" @punctuation.special)

; ============================================================================
; Instruction Markers
; ============================================================================
; Static instruction marker
"|" @punctuation.delimiter
"->" @punctuation.delimiter
