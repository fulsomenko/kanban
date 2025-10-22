---
bump: patch
---

Add markdown rendering support for task and board descriptions

- Integrate pulldown-cmark for markdown parsing
- Support bold, italic, inline code, and code blocks with proper spacing
- Code blocks render as plain text with top/bottom margins and left indent for readability
- Enhance card detail view with formatted markdown descriptions
- Enhance board detail view with formatted markdown descriptions
- Add comprehensive integration tests for markdown renderer (9 tests)
- Note: Chose markdown-only approach over syntax highlighting to maintain simplicity and performance
