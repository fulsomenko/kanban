use kanban_tui::markdown_renderer::render_markdown;

#[test]
fn test_plain_text() {
    let text = "This is plain text";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}

#[test]
fn test_bold_text() {
    let text = "This is **bold** text";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}

#[test]
fn test_italic_text() {
    let text = "This is *italic* text";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}

#[test]
fn test_code_block() {
    let text = "```rust\nfn main() {}\n```";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}

#[test]
fn test_multiple_paragraphs() {
    let text = "First paragraph\n\nSecond paragraph";
    let lines = render_markdown(text);
    assert!(lines.len() >= 3);
}

#[test]
fn test_inline_code() {
    let text = "Use `fn main()` to start";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}

#[test]
fn test_empty_text() {
    let text = "";
    let lines = render_markdown(text);
    assert!(lines.is_empty() || lines.iter().all(|line| line.spans.is_empty()));
}

#[test]
fn test_code_block_with_language() {
    let text = "```python\nprint('hello')\n```";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}

#[test]
fn test_mixed_formatting() {
    let text = "This is **bold with `code`** and *italic* text";
    let lines = render_markdown(text);
    assert!(!lines.is_empty());
}
