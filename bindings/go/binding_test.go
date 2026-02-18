package tree_sitter_wpl_test

import (
	"testing"

	tree_sitter "github.com/smacker/go-tree-sitter"
	"github.com/tree-sitter/tree-sitter-wpl"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_wpl.Language())
	if language == nil {
		t.Errorf("Error loading Wpl grammar")
	}
}
