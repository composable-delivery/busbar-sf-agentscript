; Code folding for AgentScript
; Defines which blocks can be collapsed in editors

; Top-level blocks
(config_block) @fold
(variables_block) @fold
(system_block) @fold
(start_agent_block) @fold
(topic_block) @fold
(connection_block) @fold
(language_block) @fold
(knowledge_block) @fold

; Nested blocks
(actions_block) @fold
(inputs_block) @fold
(outputs_block) @fold
(reasoning_block) @fold
(before_reasoning_block) @fold
(after_reasoning_block) @fold
(messages_block) @fold
(instructions_block) @fold
(reasoning_actions_block) @fold

; Definitions with nested content
(action_def) @fold
(variable_def) @fold
(param_def) @fold
(reasoning_action) @fold

; Control flow
(if_statement) @fold
(if_clause) @fold
(else_clause) @fold
(run_clause) @fold
(run_statement) @fold
