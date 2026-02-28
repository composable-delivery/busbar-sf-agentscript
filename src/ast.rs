//! Abstract Syntax Tree types for AgentScript.
//!
//! This module defines all types representing a parsed AgentScript file.
//! Every node in the AST is wrapped in [`Spanned`] to track its source location,
//! enabling precise error reporting and IDE features like go-to-definition.
//!
//! For a comprehensive guide to the AST structure, node relationships, and UX development patterns,
//! see the **[AST Reference](https://github.com/composable-delivery/sf-agentscript/blob/main/AST_REFERENCE.md)**.
//!
//! # AST Structure
//!
//! The root type is [`AgentFile`], which contains optional blocks:
//!
//! ```text
//! AgentFile
//! ├── config: ConfigBlock (agent metadata)
//! ├── system: SystemBlock (global instructions/messages)
//! ├── variables: VariablesBlock (state management)
//! ├── connections: ConnectionsBlock (escalation routing)
//! ├── language: LanguageBlock (locale settings)
//! ├── start_agent: StartAgentBlock (entry point)
//! └── topics: Vec<TopicBlock> (conversation topics)
//! ```
//!
//! # Span Tracking
//!
//! All nodes use [`Spanned<T>`] to preserve source locations:
//!
//! ```rust
//! use busbar_sf_agentscript::Spanned;
//!
//! // A spanned string with byte offsets 10..20
//! let name = Spanned::new("MyAgent".to_string(), 10..20);
//! assert_eq!(name.node, "MyAgent");
//! assert_eq!(name.span, 10..20);
//! ```
//!
//! # Serialization
//!
//! All types implement `Serialize` and `Deserialize` for JSON interop:
//!
//! ```rust
//! # use busbar_sf_agentscript::parse;
//! let source = "config:\n   agent_name: \"Test\"\n";
//! let agent = parse(source).unwrap();
//! let json = serde_json::to_string(&agent).unwrap();
//! ```

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// A span in the source code represented as byte offsets.
///
/// Used to track the exact location of every AST node in the original source,
/// enabling precise error messages and IDE features.
///
/// # Example
///
/// ```rust
/// use busbar_sf_agentscript::ast::Span;
///
/// let span: Span = 0..10; // Bytes 0 through 9
/// assert_eq!(span.start, 0);
/// assert_eq!(span.end, 10);
/// ```
pub type Span = Range<usize>;

/// A value with an associated source span.
///
/// This wrapper type preserves source location information throughout the AST,
/// enabling features like:
/// - Precise error messages pointing to exact source locations
/// - IDE go-to-definition and hover information
/// - Source maps for debugging
///
/// # Example
///
/// ```rust
/// use busbar_sf_agentscript::Spanned;
///
/// let spanned = Spanned::new("hello".to_string(), 5..10);
/// assert_eq!(spanned.node, "hello");
/// assert_eq!(spanned.span.start, 5);
///
/// // Transform the inner value while preserving span
/// let upper = spanned.map(|s| s.to_uppercase());
/// assert_eq!(upper.node, "HELLO");
/// assert_eq!(upper.span.start, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    /// The wrapped value.
    pub node: T,
    /// Source location as byte offsets.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new spanned value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_agentscript::Spanned;
    ///
    /// let s = Spanned::new(42, 0..2);
    /// assert_eq!(s.node, 42);
    /// ```
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    /// Transform the inner value while preserving the span.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_agentscript::Spanned;
    ///
    /// let s = Spanned::new(5, 0..1);
    /// let doubled = s.map(|n| n * 2);
    /// assert_eq!(doubled.node, 10);
    /// assert_eq!(doubled.span, 0..1);
    /// ```
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

// ============================================================================
// Top-Level File Structure
// ============================================================================

/// A complete parsed AgentScript file.
///
/// This is the root type returned by [`crate::parse()`]. It contains all the
/// top-level blocks that make up an AgentScript definition.
///
/// # Structure
///
/// An AgentScript file typically contains:
/// - `config:` - Required agent metadata
/// - `system:` - Global instructions and messages
/// - `variables:` - State variables
/// - `start_agent:` - Entry point for the agent
/// - One or more `topic:` blocks - Conversation topics
///
/// # Example
///
/// ```rust
/// use busbar_sf_agentscript::parse;
///
/// let source = r#"
/// config:
///    agent_name: "MyAgent"
///
/// topic main:
///    description: "Main topic"
/// "#;
///
/// let agent = parse(source).unwrap();
/// assert!(agent.config.is_some());
/// assert_eq!(agent.topics.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentFile {
    /// Agent configuration block (`config:` section).
    ///
    /// Contains agent metadata like name, label, and description.
    /// Should be the first block in the file.
    pub config: Option<Spanned<ConfigBlock>>,

    /// Variable declarations (`variables:` section).
    ///
    /// Defines state variables that can be `mutable` (read-write)
    /// or `linked` (read-only from external context).
    pub variables: Option<Spanned<VariablesBlock>>,

    /// System configuration (`system:` section).
    ///
    /// Contains global instructions and system messages like
    /// welcome and error messages.
    pub system: Option<Spanned<SystemBlock>>,

    /// Connection configurations (`connection <name>:` blocks).
    ///
    /// Each connection block defines escalation routing for a specific channel.
    /// Multiple connection blocks can exist at the top level.
    pub connections: Vec<Spanned<ConnectionBlock>>,

    /// Knowledge base configuration (`knowledge:` section).
    ///
    /// Configures access to external knowledge sources.
    pub knowledge: Option<Spanned<KnowledgeBlock>>,

    /// Language/locale settings (`language:` section).
    ///
    /// Configures internationalization settings.
    pub language: Option<Spanned<LanguageBlock>>,

    /// Entry point (`start_agent:` section).
    ///
    /// The initial topic selector that routes conversations
    /// to appropriate topics.
    pub start_agent: Option<Spanned<StartAgentBlock>>,

    /// Conversation topics (`topic:` sections).
    ///
    /// Each topic defines a conversational context with its own
    /// reasoning instructions and available actions.
    pub topics: Vec<Spanned<TopicBlock>>,
}

impl AgentFile {
    /// Create a new empty AgentFile.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_agentscript::AgentFile;
    ///
    /// let agent = AgentFile::new();
    /// assert!(agent.config.is_none());
    /// assert!(agent.topics.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Config Block
// ============================================================================

/// Agent configuration block containing metadata.
///
/// This block should appear first in the file and defines identifying
/// information about the agent.
///
/// # AgentScript Syntax
///
/// ```text
/// config:
///    agent_name: "CustomerSupport"
///    agent_label: "Customer Support Agent"
///    description: "Handles customer support inquiries"
///    agent_type: "AgentforceServiceAgent"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigBlock {
    /// The agent's unique identifier name (required).
    ///
    /// This is used internally to reference the agent.
    pub agent_name: Spanned<String>,

    /// Human-readable display label (optional).
    ///
    /// Shown in UIs when the agent_name is too technical.
    pub agent_label: Option<Spanned<String>>,

    /// Description of the agent's purpose.
    pub description: Option<Spanned<String>>,

    /// Agent type classification (e.g., "AgentforceServiceAgent").
    pub agent_type: Option<Spanned<String>>,

    /// Default user email for agent operations.
    pub default_agent_user: Option<Spanned<String>>,
}

// ============================================================================
// Variables Block
// ============================================================================

/// Variables block containing state variable declarations.
///
/// Variables in AgentScript can be either `mutable` (read-write) or `linked`
/// (read-only from external context).
///
/// # AgentScript Syntax
///
/// ```text
/// variables:
///    customer_name: mutable string = ""
///       description: "Customer's name"
///    order_total: linked number
///       source: @context.order.total
///       description: "Total order amount"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariablesBlock {
    /// List of variable declarations.
    pub variables: Vec<Spanned<VariableDecl>>,
}

/// A single variable declaration.
///
/// Variables have a name, kind (mutable/linked), type, and optional metadata.
/// Mutable variables require a default value; linked variables require a source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDecl {
    /// Variable name (used in references like `@variables.name`).
    pub name: Spanned<String>,

    /// Whether the variable is mutable or linked.
    pub kind: VariableKind,

    /// The variable's data type.
    pub ty: Spanned<Type>,

    /// Default value (required for mutable variables).
    ///
    /// Example: `mutable string = "default"`
    pub default: Option<Spanned<Expr>>,

    /// Human-readable description.
    pub description: Option<Spanned<String>>,

    /// Source reference for linked variables.
    ///
    /// Example: `source: @context.user.email`
    pub source: Option<Spanned<Reference>>,
}

/// Variable mutability kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableKind {
    /// Mutable variable - agent can read and write.
    ///
    /// Requires a default value in the declaration.
    Mutable,

    /// Linked variable - read-only from external context.
    ///
    /// Requires a source reference pointing to external data.
    Linked,
}

/// Data type for variables and action parameters.
///
/// AgentScript supports a fixed set of primitive types plus `List` for arrays.
///
/// # Example Types
///
/// | Type | Description |
/// |------|-------------|
/// | `string` | Text data |
/// | `number` | Floating-point number |
/// | `boolean` | True/False |
/// | `integer` | Whole number |
/// | `id` | Salesforce record ID |
/// | `list<string>` | Array of strings |
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    /// Text data (`string`).
    String,
    /// Floating-point number (`number`).
    Number,
    /// Boolean value (`boolean`).
    Boolean,
    /// Generic object (`object`).
    Object,
    /// Date without time (`date`).
    Date,
    /// Unix timestamp (`timestamp`).
    Timestamp,
    /// Currency value (`currency`).
    Currency,
    /// Salesforce record ID (`id`).
    Id,
    /// Date with time (`datetime`).
    Datetime,
    /// Time without date (`time`).
    Time,
    /// Whole number (`integer`).
    Integer,
    /// Large integer (`long`).
    Long,
    /// Array type (`list<T>`).
    List(Box<Type>),
}

impl Type {
    /// Parse a type from its string representation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_agentscript::Type;
    ///
    /// assert_eq!(Type::parse_type("string"), Some(Type::String));
    /// assert_eq!(Type::parse_type("number"), Some(Type::Number));
    /// assert_eq!(Type::parse_type("unknown"), None);
    /// ```
    pub fn parse_type(s: &str) -> Option<Self> {
        match s {
            "string" => Some(Type::String),
            "number" => Some(Type::Number),
            "boolean" => Some(Type::Boolean),
            "object" => Some(Type::Object),
            "date" => Some(Type::Date),
            "timestamp" => Some(Type::Timestamp),
            "currency" => Some(Type::Currency),
            "id" => Some(Type::Id),
            "datetime" => Some(Type::Datetime),
            "time" => Some(Type::Time),
            "integer" => Some(Type::Integer),
            "long" => Some(Type::Long),
            _ => None,
        }
    }
}

// ============================================================================
// System Block
// ============================================================================

/// The system block defines global agent settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemBlock {
    /// System messages.
    pub messages: Option<Spanned<SystemMessages>>,
    /// System instructions.
    pub instructions: Option<Spanned<Instructions>>,
}

/// System messages (welcome, error).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessages {
    pub welcome: Option<Spanned<String>>,
    pub error: Option<Spanned<String>>,
}

// ============================================================================
// Connection Block
// ============================================================================

/// A connection block defines escalation routing for a specific channel.
///
/// # AgentScript Syntax
///
/// ```text
/// connection messaging:
///    escalation_message: "I'm connecting you with a specialist."
///    outbound_route_type: "OmniChannelFlow"
///    outbound_route_name: "SpecialistQueue"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionBlock {
    /// Connection name (e.g., "messaging").
    pub name: Spanned<String>,
    /// Connection configuration entries.
    pub entries: Vec<Spanned<ConnectionEntry>>,
}

/// A key-value entry in a connection block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionEntry {
    pub name: Spanned<String>,
    pub value: Spanned<String>,
}

// Legacy type alias for backwards compatibility
// TODO: Remove in next major version
/// @deprecated Use ConnectionBlock instead - this type exists only for backwards compatibility
pub type ConnectionsBlock = ConnectionBlock;

// ============================================================================
// Knowledge Block
// ============================================================================

/// The knowledge block configures knowledge base access.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeBlock {
    /// Knowledge configuration entries.
    pub entries: Vec<Spanned<KnowledgeEntry>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub name: Spanned<String>,
    pub value: Spanned<Expr>,
}

// ============================================================================
// Language Block
// ============================================================================

/// The language block configures locale settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageBlock {
    /// Language setting entries.
    pub entries: Vec<Spanned<LanguageEntry>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageEntry {
    pub name: Spanned<String>,
    pub value: Spanned<Expr>,
}

// ============================================================================
// Start Agent Block
// ============================================================================

/// The start_agent block is the entry point for the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartAgentBlock {
    /// The name after start_agent (typically "topic_selector").
    pub name: Spanned<String>,
    /// Description of the start agent.
    pub description: Option<Spanned<String>>,
    /// Optional system override.
    pub system: Option<Spanned<TopicSystemOverride>>,
    /// Optional action definitions.
    pub actions: Option<Spanned<ActionsBlock>>,
    /// Optional before_reasoning block.
    pub before_reasoning: Option<Spanned<DirectiveBlock>>,
    /// The reasoning block (required).
    pub reasoning: Option<Spanned<ReasoningBlock>>,
    /// Optional after_reasoning block.
    pub after_reasoning: Option<Spanned<DirectiveBlock>>,
}

// ============================================================================
// Topic Block
// ============================================================================

/// A topic block defines a conversation topic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicBlock {
    /// Topic name.
    pub name: Spanned<String>,
    /// Description of the topic.
    pub description: Option<Spanned<String>>,
    /// Optional system override.
    pub system: Option<Spanned<TopicSystemOverride>>,
    /// Optional action definitions.
    pub actions: Option<Spanned<ActionsBlock>>,
    /// Optional before_reasoning block.
    pub before_reasoning: Option<Spanned<DirectiveBlock>>,
    /// The reasoning block (required).
    pub reasoning: Option<Spanned<ReasoningBlock>>,
    /// Optional after_reasoning block.
    pub after_reasoning: Option<Spanned<DirectiveBlock>>,
}

/// System instruction override for a topic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicSystemOverride {
    pub instructions: Option<Spanned<Instructions>>,
}

// ============================================================================
// Actions Block
// ============================================================================

/// A block of action definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionsBlock {
    pub actions: Vec<Spanned<ActionDef>>,
}

/// An action definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    /// Action name.
    pub name: Spanned<String>,
    /// Description.
    pub description: Option<Spanned<String>>,
    /// Display label.
    pub label: Option<Spanned<String>>,
    /// Whether user confirmation is required.
    pub require_user_confirmation: Option<Spanned<bool>>,
    /// Whether to show progress indicator.
    pub include_in_progress_indicator: Option<Spanned<bool>>,
    /// Progress indicator message.
    pub progress_indicator_message: Option<Spanned<String>>,
    /// Input parameters.
    pub inputs: Option<Spanned<Vec<Spanned<ParamDef>>>>,
    /// Output parameters.
    pub outputs: Option<Spanned<Vec<Spanned<ParamDef>>>>,
    /// Target (e.g., "flow://FlowName").
    pub target: Option<Spanned<String>>,
}

/// A parameter definition (for action inputs/outputs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    /// Parameter name.
    pub name: Spanned<String>,
    /// Parameter type.
    pub ty: Spanned<Type>,
    /// Description.
    pub description: Option<Spanned<String>>,
    /// Display label.
    pub label: Option<Spanned<String>>,
    /// Whether required (for inputs).
    pub is_required: Option<Spanned<bool>>,
    /// Whether to filter from agent.
    pub filter_from_agent: Option<Spanned<bool>>,
    /// Whether displayable.
    pub is_displayable: Option<Spanned<bool>>,
    /// Complex data type name.
    pub complex_data_type_name: Option<Spanned<String>>,
}

// ============================================================================
// Directive Blocks (before_reasoning, after_reasoning)
// ============================================================================

/// A directive block contains imperative statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectiveBlock {
    pub statements: Vec<Spanned<Stmt>>,
}

/// A statement in a directive block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    /// Variable assignment: `set @variables.x = expr`
    Set {
        target: Spanned<Reference>,
        value: Spanned<Expr>,
    },
    /// Action invocation: `run @actions.foo`
    Run {
        action: Spanned<Reference>,
        with_clauses: Vec<Spanned<WithClause>>,
        set_clauses: Vec<Spanned<SetClause>>,
    },
    /// Conditional: `if expr:`
    If {
        condition: Spanned<Expr>,
        then_block: Vec<Spanned<Stmt>>,
        else_block: Option<Vec<Spanned<Stmt>>>,
    },
    /// Transition: `transition to @topic.name`
    Transition { target: Spanned<Reference> },
}

/// A with clause binds an action input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithClause {
    pub param: Spanned<String>,
    pub value: Spanned<WithValue>,
}

/// The value of a with clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WithValue {
    /// A concrete expression.
    Expr(Expr),
}

/// A set clause assigns a value to a variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetClause {
    pub target: Spanned<Reference>,
    /// The value to assign (can be reference, literal, or expression).
    pub source: Spanned<Expr>,
}

/// Reference to an action output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputRef {
    /// The output name (e.g., "result" from "@outputs.result").
    pub name: String,
}

// ============================================================================
// Reasoning Block
// ============================================================================

/// The reasoning block configures LLM reasoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningBlock {
    /// Instructions for the LLM.
    pub instructions: Option<Spanned<Instructions>>,
    /// Actions available to the LLM.
    pub actions: Option<Spanned<Vec<Spanned<ReasoningAction>>>>,
}

/// An action available during reasoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningAction {
    /// Action alias name.
    pub name: Spanned<String>,
    /// Target reference (e.g., `@actions.foo` or `@utils.transition to @topic.x`).
    pub target: Spanned<ReasoningActionTarget>,
    /// Optional description override.
    pub description: Option<Spanned<String>>,
    /// Availability condition.
    pub available_when: Option<Spanned<Expr>>,
    /// Input bindings.
    pub with_clauses: Vec<Spanned<WithClause>>,
    /// Output captures.
    pub set_clauses: Vec<Spanned<SetClause>>,
    /// Chained run statements.
    pub run_clauses: Vec<Spanned<RunClause>>,
    /// Post-action if conditions.
    pub if_clauses: Vec<Spanned<IfClause>>,
    /// Post-action transition.
    pub transition: Option<Spanned<Reference>>,
}

/// Target of a reasoning action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReasoningActionTarget {
    /// Reference to an action: `@actions.foo`
    Action(Reference),
    /// Transition utility: `@utils.transition to @topic.x`
    TransitionTo(Reference),
    /// Escalate utility: `@utils.escalate`
    Escalate,
    /// Set variables utility: `@utils.setVariables`
    SetVariables,
    /// Topic delegation: `@topic.x`
    TopicDelegate(Reference),
}

/// A chained run clause in reasoning actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunClause {
    pub action: Spanned<Reference>,
    pub with_clauses: Vec<Spanned<WithClause>>,
    pub set_clauses: Vec<Spanned<SetClause>>,
}

/// A conditional clause in reasoning actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfClause {
    pub condition: Spanned<Expr>,
    pub transition: Option<Spanned<Reference>>,
}

// ============================================================================
// Instructions
// ============================================================================

/// Instructions can be static or dynamic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instructions {
    /// Simple string instructions.
    Simple(String),
    /// Static multiline (`:| ...`).
    Static(Vec<Spanned<String>>),
    /// Dynamic with conditionals (`:-> ...`).
    Dynamic(Vec<Spanned<InstructionPart>>),
}

/// A part of dynamic instructions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstructionPart {
    /// Literal text line (prefixed with `|`).
    Text(String),
    /// Template interpolation `{!expr}`.
    Interpolation(Expr),
    /// Conditional section.
    Conditional {
        condition: Spanned<Expr>,
        then_parts: Vec<Spanned<InstructionPart>>,
        else_parts: Option<Vec<Spanned<InstructionPart>>>,
    },
}

// ============================================================================
// Expressions
// ============================================================================

/// An expression in AgentScript.
///
/// Expressions can appear in conditions, variable assignments, action bindings,
/// and dynamic instruction interpolation.
///
/// # Variants
///
/// | Variant | Example | Description |
/// |---------|---------|-------------|
/// | `Reference` | `@variables.name` | Reference to a namespaced resource |
/// | `String` | `"hello"` | String literal |
/// | `Number` | `42`, `3.14` | Numeric literal |
/// | `Bool` | `True`, `False` | Boolean literal |
/// | `None` | `None` | Null value |
/// | `List` | `[1, 2, 3]` | Array literal |
/// | `Object` | `{key: value}` | Object literal |
/// | `BinOp` | `a == b` | Binary operation |
/// | `UnaryOp` | `not x` | Unary operation |
/// | `Ternary` | `x if cond else y` | Conditional expression |
/// | `Property` | `obj.field` | Property access |
/// | `Index` | `arr[0]` | Index access |
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Reference to a namespaced resource: `@namespace.path`.
    ///
    /// Common namespaces: `variables`, `actions`, `outputs`, `topic`, `utils`.
    Reference(Reference),

    /// String literal: `"text"`.
    String(String),

    /// Numeric literal: `42` or `3.14`.
    Number(f64),

    /// Boolean literal: `True` or `False`.
    Bool(bool),

    /// Null value: `None`.
    None,

    /// Array literal: `[item1, item2, ...]`.
    List(Vec<Spanned<Expr>>),

    /// Object literal: `{key1: value1, key2: value2}`.
    Object(IndexMap<String, Spanned<Expr>>),

    /// Binary operation: `left op right`.
    BinOp {
        /// Left operand.
        left: Box<Spanned<Expr>>,
        /// Operator.
        op: BinOp,
        /// Right operand.
        right: Box<Spanned<Expr>>,
    },

    /// Unary operation: `op operand`.
    UnaryOp {
        /// Operator (e.g., `not`, `-`).
        op: UnaryOp,
        /// Operand.
        operand: Box<Spanned<Expr>>,
    },

    /// Ternary conditional: `then_expr if condition else else_expr`.
    ///
    /// Note: Python-style ordering (value first, then condition).
    Ternary {
        /// The condition to evaluate.
        condition: Box<Spanned<Expr>>,
        /// Value if condition is true.
        then_expr: Box<Spanned<Expr>>,
        /// Value if condition is false.
        else_expr: Box<Spanned<Expr>>,
    },

    /// Property access: `object.field`.
    Property {
        /// The object to access.
        object: Box<Spanned<Expr>>,
        /// The field name.
        field: Spanned<String>,
    },

    /// Index access: `object[index]`.
    Index {
        /// The array/object to index.
        object: Box<Spanned<Expr>>,
        /// The index expression.
        index: Box<Spanned<Expr>>,
    },
}

/// A reference to a namespaced resource.
///
/// References use the `@namespace.path` syntax to access variables, actions,
/// topics, and utilities.
///
/// # Common Namespaces
///
/// | Namespace | Purpose | Example |
/// |-----------|---------|---------|
/// | `variables` | State variables | `@variables.customer_id` |
/// | `actions` | Action definitions | `@actions.lookup_order` |
/// | `outputs` | Action outputs | `@outputs.result` |
/// | `topic` | Topic references | `@topic.support` |
/// | `utils` | Built-in utilities | `@utils.transition` |
/// | `context` | External context | `@context.user.email` |
///
/// # Example
///
/// ```rust
/// use busbar_sf_agentscript::Reference;
///
/// let ref1 = Reference::new("variables", vec!["customer_id".to_string()]);
/// assert_eq!(ref1.full_path(), "@variables.customer_id");
///
/// let ref2 = Reference::new("utils", vec!["transition".to_string()]);
/// assert_eq!(ref2.full_path(), "@utils.transition");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Reference {
    /// The namespace (e.g., "variables", "actions", "outputs", "utils", "topic").
    pub namespace: String,

    /// Path components after the namespace.
    ///
    /// For `@variables.user.name`, this would be `["user", "name"]`.
    pub path: Vec<String>,
}

impl Reference {
    /// Create a new reference.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_agentscript::Reference;
    ///
    /// let r = Reference::new("variables", vec!["name".to_string()]);
    /// assert_eq!(r.namespace, "variables");
    /// ```
    pub fn new(namespace: impl Into<String>, path: Vec<String>) -> Self {
        Self {
            namespace: namespace.into(),
            path,
        }
    }

    /// Get the full reference path as a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_agentscript::Reference;
    ///
    /// let r = Reference::new("actions", vec!["send_email".to_string()]);
    /// assert_eq!(r.full_path(), "@actions.send_email");
    /// ```
    pub fn full_path(&self) -> String {
        if self.path.is_empty() {
            format!("@{}", self.namespace)
        } else {
            format!("@{}.{}", self.namespace, self.path.join("."))
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    // Comparison
    Eq,    // ==
    Ne,    // !=
    Lt,    // <
    Gt,    // >
    Le,    // <=
    Ge,    // >=
    Is,    // is
    IsNot, // is not

    // Logical
    And, // and
    Or,  // or

    // Arithmetic
    Add, // +
    Sub, // -
}

impl BinOp {
    /// Get the precedence of this operator (higher binds tighter).
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Is | BinOp::IsNot => 3,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => 4,
            BinOp::Add | BinOp::Sub => 5,
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not, // not
    Neg, // - (negative)
}

// ============================================================================
// Comments
// ============================================================================

/// A comment in the source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub text: String,
    pub span: Span,
}
