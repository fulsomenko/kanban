#[derive(Debug, Clone, PartialEq, Default)]
pub enum Focus {
    #[default]
    Boards,
    Cards,
}

#[derive(Default)]
pub struct FocusState {
    pub active: Focus,
    pub card_focus: CardFocus,
    pub board_focus: BoardFocus,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CardFocus {
    #[default]
    Title,
    Metadata,
    Description,
    Parents,
    Children,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BoardFocus {
    #[default]
    Name,
    Description,
    Settings,
    Sprints,
    Columns,
}
