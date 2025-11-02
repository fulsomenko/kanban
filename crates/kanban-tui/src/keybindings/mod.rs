pub mod normal_mode;
pub mod card_detail;
pub mod board_detail;
pub mod sprint_detail;
pub mod dialog_modes;
pub mod registry;

pub use registry::KeybindingRegistry;

#[derive(Debug, Clone)]
pub struct Keybinding {
    pub key: String,
    pub description: String,
}

impl Keybinding {
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeybindingContext {
    pub name: String,
    pub bindings: Vec<Keybinding>,
}

impl KeybindingContext {
    pub fn new(name: impl Into<String>, bindings: Vec<Keybinding>) -> Self {
        Self {
            name: name.into(),
            bindings,
        }
    }
}

pub trait KeybindingProvider {
    fn get_context(&self) -> KeybindingContext;
}
