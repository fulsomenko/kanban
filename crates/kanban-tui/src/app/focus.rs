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
    pub settings_focus: SettingsFocus,
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SettingsFocus {
    #[default]
    Configuration,
    ConfigFile,
    Storage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_focus_default_is_configuration() {
        let focus = SettingsFocus::default();
        assert_eq!(focus, SettingsFocus::Configuration);
    }
}
