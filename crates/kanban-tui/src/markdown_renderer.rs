use pulldown_cmark::{CowStr, Event, Parser, Tag, TagEnd};
use ratatui::prelude::Stylize;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let parser = Parser::new(text);
    let mut renderer = MarkdownRenderer::new();

    for event in parser {
        renderer.process_event(event);
    }

    renderer.finish()
}

struct MarkdownRenderer {
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    in_code_block: bool,
    code_block_lang: String,
    code_block_content: String,
    in_emphasis: bool,
    in_strong: bool,
}

impl MarkdownRenderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_line: Vec::new(),
            in_code_block: false,
            code_block_lang: String::new(),
            code_block_content: String::new(),
            in_emphasis: false,
            in_strong: false,
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_tag_start(tag),
            Event::End(tag_end) => self.handle_tag_end(tag_end),
            Event::Text(text) => self.handle_text(text),
            Event::Code(code) => self.handle_inline_code(code),
            Event::SoftBreak | Event::HardBreak => self.handle_break(),
            _ => {}
        }
    }

    fn handle_tag_start(&mut self, tag: Tag) {
        match tag {
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Tag::Emphasis => {
                self.in_emphasis = true;
            }
            Tag::Strong => {
                self.in_strong = true;
            }
            Tag::Paragraph | Tag::Heading { .. } => {
                if !self.current_line.is_empty() {
                    self.flush_line();
                }
            }
            Tag::List(_) => {}
            Tag::Item => {}
            _ => {}
        }
    }

    fn handle_tag_end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.render_code_block();
            }
            TagEnd::Emphasis => {
                self.in_emphasis = false;
            }
            TagEnd::Strong => {
                self.in_strong = false;
            }
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item => {
                if !self.current_line.is_empty() {
                    self.flush_line();
                }
                self.lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: CowStr) {
        if self.in_code_block {
            self.code_block_content.push_str(&text);
        } else {
            let mut style = Style::default();
            if self.in_strong {
                style = style.bold();
            }
            if self.in_emphasis {
                style = style.italic();
            }

            let text_str = text.to_string();
            self.current_line.push(Span::styled(text_str, style));
        }
    }

    fn handle_inline_code(&mut self, code: CowStr) {
        let style = Style::default().italic();
        self.current_line
            .push(Span::styled(format!("`{}`", code), style));
    }

    fn handle_break(&mut self) {
        self.flush_line();
    }

    fn render_code_block(&mut self) {
        if self.code_block_content.is_empty() {
            return;
        }

        let highlighted_lines = highlight_code(&self.code_block_lang, &self.code_block_content);

        for line_spans in highlighted_lines {
            self.lines.push(Line::from(line_spans));
        }

        self.code_block_content.clear();
    }

    fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            let line = Line::from(std::mem::take(&mut self.current_line));
            self.lines.push(line);
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line();
        self.lines
    }
}

fn highlight_code(_language: &str, code: &str) -> Vec<Vec<Span<'static>>> {
    let mut result = Vec::new();
    for line in code.lines() {
        let mut line_spans = Vec::new();
        line_spans.push(Span::raw(line.to_string()));
        result.push(line_spans);
    }
    result
}
