//! This crate provides Wpl language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [language][language func] function to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! parser.set_language(&tree_sitter_wpl::language()).expect("Error loading Wpl grammar");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
//! [language func]: fn.language.html
//! [Parser]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter::Language;

#[path = "../../src/format/mod.rs"]
mod format;

extern "C" {
    fn tree_sitter_wpl() -> Language;
}

/// Get the tree-sitter [Language][] for this grammar.
///
/// [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
pub fn language() -> Language {
    unsafe { tree_sitter_wpl() }
}

/// The content of the [`node-types.json`][] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");
pub const COMPLETION_BUNDLE: &str = include_str!("../../completions/completion.bundle.json");
pub const EDITOR_ASSET_MANIFEST: &str = include_str!("../../editor/asset-manifest.json");

pub use format::{format, format_or_original, format_with_indent, WplFormatError, WplFormatter};
// pub const INJECTIONS_QUERY: &str = include_str!("../../queries/injections.scm");
// pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");
// pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    use tree_sitter::Query;

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading Wpl grammar");
    }

    #[test]
    fn test_highlights_query_compiles() {
        Query::new(&super::language(), super::HIGHLIGHTS_QUERY)
            .expect("highlights query should compile");
    }

    #[test]
    fn test_editor_assets_exported() {
        assert!(super::COMPLETION_BUNDLE.contains("\"language\": \"wpl\""));
        assert!(super::EDITOR_ASSET_MANIFEST.contains("\"language_id\": \"wpl\""));
        assert!(super::EDITOR_ASSET_MANIFEST.contains("\"parser_wasm\": \"editor/wasm/tree-sitter-wpl.wasm\""));
        assert!(super::EDITOR_ASSET_MANIFEST.contains("\"completion_bundle\": \"completions/completion.bundle.json\""));
    }
}
