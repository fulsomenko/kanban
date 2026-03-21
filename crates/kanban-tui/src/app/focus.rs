#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Boards,
    Cards,
}

pub struct FocusState {
    pub focus: Focus,
    pub card_focus: CardFocus,
    pub board_focus: BoardFocus,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            focus: Focus::Boards,
            card_focus: CardFocus::Title,
            board_focus: BoardFocus::Name,
        }
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardFocus {
    Title,
    Metadata,
    Description,
    Parents,
    Children,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoardFocus {
    Name,
    Description,
    Settings,
    Sprints,
    Columns,
}
