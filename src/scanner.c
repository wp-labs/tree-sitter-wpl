// External scanner for WPL tree-sitter grammar.
// Handles quote_format: a standalone " that acts as a format marker.
// Distinguished from quoted_string by context: quote_format is always
// followed by a delimiter character (, ) | \ { whitespace EOF).
// quoted_string is handled as a single token by the grammar.

#include "tree_sitter/parser.h"
#include <stdlib.h>

enum TokenType {
  QUOTE_FORMAT,
};

void *tree_sitter_wpl_external_scanner_create(void) {
  return NULL;
}

void tree_sitter_wpl_external_scanner_destroy(void *payload) {
  (void)payload;
}

unsigned tree_sitter_wpl_external_scanner_serialize(void *payload,
                                                     char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}

void tree_sitter_wpl_external_scanner_deserialize(void *payload,
                                                    const char *buffer,
                                                    unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

bool tree_sitter_wpl_external_scanner_scan(void *payload, TSLexer *lexer,
                                            const bool *valid_symbols) {
  (void)payload;

  if (!valid_symbols[QUOTE_FORMAT]) {
    return false;
  }

  if (lexer->lookahead != '"') {
    return false;
  }

  // Mark position to check what follows the "
  lexer->mark_end(lexer);
  lexer->advance(lexer, false);
  int32_t next = lexer->lookahead;

  // quote_format: " followed by a delimiter character
  if (next == ',' || next == ')' || next == '|' || next == '\\' ||
      next == '{' || next == ' ' || next == '\t' || next == '\n' ||
      next == '\r' || next == 0) {
    lexer->mark_end(lexer);
    lexer->result_symbol = QUOTE_FORMAT;
    return true;
  }

  // Not a quote_format - don't consume the "
  return false;
}
