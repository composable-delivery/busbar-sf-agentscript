/**
 * External scanner for AgentScript indentation handling.
 *
 * AgentScript uses significant whitespace (3-space indentation).
 * This scanner tracks indent levels and emits INDENT/DEDENT/NEWLINE tokens.
 *
 * Based on tree-sitter-python's approach.
 */

#include "tree_sitter/parser.h"
#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

// Enable debug output
#define DEBUG 0

#if DEBUG
#define LOG(...) fprintf(stderr, __VA_ARGS__)
#else
#define LOG(...)
#endif

// Token types - must match order in grammar.js externals
enum TokenType {
    NEWLINE,
    INDENT,
    DEDENT,
    INTERPOLATION_START,       // {!
    INSTRUCTION_TEXT_SEGMENT,  // text that doesn't contain {! or newline
};

// Maximum indent depth
#define MAX_INDENT_DEPTH 100

typedef struct {
    uint16_t indents[MAX_INDENT_DEPTH];
    uint8_t indent_count;
    // Track pending indent level after newline processing
    int16_t pending_indent;  // -1 means no pending, otherwise the target indent level
} Scanner;

// Helper to advance and include in token
static inline void advance(TSLexer *lexer) {
    lexer->advance(lexer, false);
}

// Helper to advance without including in token
static inline void skip(TSLexer *lexer) {
    lexer->advance(lexer, true);
}

// Allocate scanner state
void *tree_sitter_agentscript_external_scanner_create(void) {
    Scanner *scanner = calloc(1, sizeof(Scanner));
    // Start with base indent level of 0
    scanner->indents[0] = 0;
    scanner->indent_count = 1;
    scanner->pending_indent = -1;  // No pending indent
    LOG("Scanner created, indent_count=%d\n", scanner->indent_count);
    return scanner;
}

// Free scanner state
void tree_sitter_agentscript_external_scanner_destroy(void *payload) {
    free(payload);
}

// Serialize scanner state for incremental parsing
unsigned tree_sitter_agentscript_external_scanner_serialize(void *payload, char *buffer) {
    Scanner *scanner = (Scanner *)payload;
    size_t size = 0;

    buffer[size++] = (char)scanner->indent_count;
    // Serialize pending_indent as 2 bytes
    buffer[size++] = scanner->pending_indent & 0xFF;
    buffer[size++] = (scanner->pending_indent >> 8) & 0xFF;

    for (uint8_t i = 0; i < scanner->indent_count && size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE - 2; i++) {
        buffer[size++] = scanner->indents[i] & 0xFF;
        buffer[size++] = (scanner->indents[i] >> 8) & 0xFF;
    }

    return size;
}

// Deserialize scanner state
void tree_sitter_agentscript_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    Scanner *scanner = (Scanner *)payload;

    if (length == 0) {
        scanner->indents[0] = 0;
        scanner->indent_count = 1;
        scanner->pending_indent = -1;
        return;
    }

    size_t size = 0;
    scanner->indent_count = (uint8_t)buffer[size++];
    if (scanner->indent_count > MAX_INDENT_DEPTH) {
        scanner->indent_count = MAX_INDENT_DEPTH;
    }

    // Deserialize pending_indent
    if (size + 1 < length) {
        scanner->pending_indent = (uint8_t)buffer[size++];
        scanner->pending_indent |= ((int16_t)(int8_t)buffer[size++]) << 8;
    } else {
        scanner->pending_indent = -1;
    }

    for (uint8_t i = 0; i < scanner->indent_count && size + 1 < length; i++) {
        scanner->indents[i] = (uint8_t)buffer[size++];
        scanner->indents[i] |= ((uint16_t)(uint8_t)buffer[size++]) << 8;
    }

    // Ensure we have at least base level
    if (scanner->indent_count == 0) {
        scanner->indents[0] = 0;
        scanner->indent_count = 1;
    }
}

// Main scanning function
bool tree_sitter_agentscript_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    Scanner *scanner = (Scanner *)payload;

    LOG("scan: lookahead='%c' (%d), valid=[N=%d,I=%d,D=%d,IS=%d,ITS=%d], indent_count=%d, current_indent=%d, pending=%d\n",
        lexer->lookahead > 31 ? lexer->lookahead : '?',
        lexer->lookahead,
        valid_symbols[NEWLINE],
        valid_symbols[INDENT],
        valid_symbols[DEDENT],
        valid_symbols[INTERPOLATION_START],
        valid_symbols[INSTRUCTION_TEXT_SEGMENT],
        scanner->indent_count,
        scanner->indents[scanner->indent_count - 1],
        scanner->pending_indent);

    // Handle interpolation tokens (for dynamic instructions)
    // Check for {! (interpolation start)
    if (valid_symbols[INTERPOLATION_START] && lexer->lookahead == '{') {
        advance(lexer);
        LOG("  after {, lookahead='%c' (%d)\n", lexer->lookahead > 31 ? lexer->lookahead : '?', lexer->lookahead);
        if (lexer->lookahead == '!') {
            advance(lexer);
            lexer->mark_end(lexer);
            lexer->result_symbol = INTERPOLATION_START;
            LOG("  => INTERPOLATION_START\n");
            return true;
        }
        // Not {! - this was just a lone {, let other rules handle it
        // But we've already consumed the {, so we need to return it as text
        LOG("  not {!, char after { is '%c'\n", lexer->lookahead);
        if (valid_symbols[INSTRUCTION_TEXT_SEGMENT]) {
            // Continue matching instruction text (includes the { we consumed)
            while (lexer->lookahead != '\0' &&
                   lexer->lookahead != '\n' &&
                   lexer->lookahead != '{') {
                advance(lexer);
            }
            lexer->mark_end(lexer);
            lexer->result_symbol = INSTRUCTION_TEXT_SEGMENT;
            LOG("  => INSTRUCTION_TEXT_SEGMENT (after lone {)\n");
            return true;
        }
        // Can't handle this, return false
        return false;
    }

    // Handle instruction text segment (text without {! or newline)
    if (valid_symbols[INSTRUCTION_TEXT_SEGMENT] &&
        lexer->lookahead != '\0' &&
        lexer->lookahead != '\n' &&
        lexer->lookahead != '{') {
        // Match characters until we hit {, newline, or EOF
        while (lexer->lookahead != '\0' &&
               lexer->lookahead != '\n' &&
               lexer->lookahead != '{') {
            advance(lexer);
        }
        lexer->mark_end(lexer);
        lexer->result_symbol = INSTRUCTION_TEXT_SEGMENT;
        LOG("  => INSTRUCTION_TEXT_SEGMENT\n");
        return true;
    }

    // Check for pending dedents first (before processing any newlines)
    // This handles the case where we've already processed a newline but
    // need to emit more DEDENTs
    if (scanner->pending_indent >= 0 && valid_symbols[DEDENT]) {
        uint16_t current_indent = scanner->indents[scanner->indent_count - 1];
        if (scanner->pending_indent < current_indent && scanner->indent_count > 1) {
            scanner->indent_count--;
            LOG("  => DEDENT (pending, back to %d)\n", scanner->indents[scanner->indent_count - 1]);
            lexer->result_symbol = DEDENT;
            // Clear pending if we've reached target level
            if (scanner->indents[scanner->indent_count - 1] <= scanner->pending_indent) {
                scanner->pending_indent = -1;
            }
            return true;
        }
        // Clear pending if no longer applicable
        scanner->pending_indent = -1;
    }

    // If we're not at a newline or EOF, don't process anything
    // Let tree-sitter's extras handle same-line whitespace
    if (lexer->lookahead != '\n' && lexer->lookahead != '\r' && !lexer->eof(lexer)) {
        LOG("  not at newline/eof, returning false\n");
        return false;
    }

    bool found_end_of_line = false;
    uint16_t indent_length = 0;

    // Process newlines and following whitespace
    // Use advance() for the first newline to give the token non-zero size
    for (;;) {
        if (lexer->lookahead == '\n') {
            if (!found_end_of_line) {
                // First newline - include in token to give it size
                advance(lexer);
                lexer->mark_end(lexer);
            } else {
                // Subsequent newlines - skip
                skip(lexer);
            }
            found_end_of_line = true;
            indent_length = 0;
            LOG("  consumed newline\n");
        } else if (lexer->lookahead == '\r') {
            skip(lexer);
            LOG("  skipped CR\n");
        } else if (lexer->lookahead == ' ' && found_end_of_line) {
            // Only count spaces AFTER we've seen a newline
            indent_length++;
            skip(lexer);
        } else if (lexer->lookahead == '\t' && found_end_of_line) {
            // Tab = 3 spaces (AgentScript standard)
            indent_length += 3;
            skip(lexer);
        } else if (lexer->lookahead == '#' && found_end_of_line) {
            // Comment on its own line - skip to end of line
            while (lexer->lookahead && lexer->lookahead != '\n') {
                skip(lexer);
            }
            LOG("  skipped comment\n");
            // Let the next iteration handle the newline
        } else if (lexer->eof(lexer)) {
            if (!found_end_of_line) {
                // At EOF without seeing newline - need to mark position
                lexer->mark_end(lexer);
            }
            indent_length = 0;
            found_end_of_line = true;
            LOG("  EOF\n");
            break;
        } else {
            // Non-whitespace character
            LOG("  found non-ws '%c', indent_length=%d\n", lexer->lookahead, indent_length);
            break;
        }
    }

    // Only emit indent-related tokens after finding end of line
    if (found_end_of_line) {
        uint16_t current_indent = scanner->indents[scanner->indent_count - 1];
        LOG("  found_eol: indent_length=%d, current_indent=%d\n", indent_length, current_indent);

        // Check for INDENT first (higher priority)
        if (valid_symbols[INDENT] && indent_length > current_indent) {
            if (scanner->indent_count < MAX_INDENT_DEPTH) {
                scanner->indents[scanner->indent_count++] = indent_length;
            }
            scanner->pending_indent = -1;  // Clear any pending
            LOG("  => INDENT (new level %d)\n", indent_length);
            lexer->result_symbol = INDENT;
            return true;
        }

        // Check for DEDENT
        if (valid_symbols[DEDENT] && indent_length < current_indent) {
            scanner->indent_count--;
            LOG("  => DEDENT (back to %d)\n", scanner->indents[scanner->indent_count - 1]);
            lexer->result_symbol = DEDENT;
            // Set pending if we need more dedents
            if (indent_length < scanner->indents[scanner->indent_count - 1]) {
                scanner->pending_indent = indent_length;
            }
            return true;
        }

        // If dedent needed but not valid, store as pending for later
        if (indent_length < current_indent) {
            scanner->pending_indent = indent_length;
            LOG("  stored pending_indent=%d\n", indent_length);
        }

        // Emit NEWLINE for same-level or when INDENT/DEDENT not applicable
        // But NOT at EOF with no content - that causes infinite loops
        if (valid_symbols[NEWLINE] && !lexer->eof(lexer)) {
            LOG("  => NEWLINE\n");
            lexer->result_symbol = NEWLINE;
            return true;
        }
    }

    LOG("  => no token\n");
    return false;
}
