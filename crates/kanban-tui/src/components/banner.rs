use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BannerVariant {
    Error,
    Success,
}

impl BannerVariant {
    fn color(self) -> Color {
        match self {
            BannerVariant::Error => Color::Red,
            BannerVariant::Success => Color::Green,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Banner {
    pub message: String,
    pub variant: BannerVariant,
    pub created_at: Instant,
}

impl Banner {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: BannerVariant::Error,
            created_at: Instant::now(),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: BannerVariant::Success,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let color = self.variant.color();

        // +4 for border chars and padding
        let box_width = (self.message.len() + 4).min(area.width as usize) as u16;
        let centered_x = if area.width > box_width {
            (area.width - box_width) / 2
        } else {
            0
        };

        let banner_area = Rect {
            x: area.x + centered_x,
            y: area.y,
            width: box_width,
            height: 3,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));

        let text_style = Style::default().fg(color).add_modifier(Modifier::BOLD);

        let widget = Paragraph::new(self.message.as_str())
            .style(text_style)
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(widget, banner_area);
    }
}
