; Scope and reference tracking for AgentScript
; Used for "go to definition" and scope-aware highlighting

; Scopes
(source_file) @scope
(topic_block) @scope
(start_agent_block) @scope
(reasoning_block) @scope
(action_def) @scope
(reasoning_action) @scope

; Definitions
(variable_def
  (identifier) @definition.var)

(param_def
  name: (identifier) @definition.parameter)

(action_def
  (identifier) @definition.function)

(reasoning_action
  name: (identifier) @definition.function)

(topic_block
  (identifier) @definition.type)

(connection_block
  (identifier) @definition.type)

; References (variables, actions, topics)
(reference) @reference
