use crate::theme::{focused_border, unfocused_border};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct PanelConfig<'a> {
    pub title: &'a str,
    pub focused_title: &'a str,
    pub is_focused: bool,
}

impl<'a> PanelConfig<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            focused_title: title,
            is_focused: false,
        }
    }

    pub fn with_focus_indicator(mut self, focused_title: &'a str) -> Self {
        self.focused_title = focused_title;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    pub fn border_style(&self) -> ratatui::style::Style {
        if self.is_focused {
            focused_border()
        } else {
            unfocused_border()
        }
    }

    pub fn title_text(&self) -> &str {
        if self.is_focused {
            self.focused_title
        } else {
            self.title
        }
    }

    pub fn block(&'a self) -> Block<'a> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(self.border_style())
            .title(self.title_text())
    }
}

pub fn render_panel<'a>(
    frame: &mut Frame,
    area: Rect,
    config: &PanelConfig<'a>,
    content: Paragraph<'a>,
) {
    let widget = content.block(config.block());
    frame.render_widget(widget, area);
}
