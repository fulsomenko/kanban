pub struct InputState {
    buffer: String,
    cursor: usize,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor].chars().next_back().unwrap();
            self.cursor -= prev.len_utf8();
            self.buffer.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor].chars().next_back().unwrap();
            self.cursor -= prev.len_utf8();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            let next = self.buffer[self.cursor..].chars().next().unwrap();
            self.cursor += next.len_utf8();
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    pub fn set(&mut self, text: String) {
        self.buffer = text;
        self.cursor = self.buffer.len();
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    pub fn cursor_pos(&self) -> usize {
        self.cursor
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let input = InputState::new();
        assert!(input.is_empty());
        assert_eq!(input.cursor_pos(), 0);
        assert_eq!(input.as_str(), "");
    }

    #[test]
    fn test_insert_char_at_end() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        assert_eq!(input.as_str(), "abc");
        assert_eq!(input.cursor_pos(), 3);
    }

    #[test]
    fn test_insert_char_at_beginning() {
        let mut input = InputState::new();
        input.insert_char('b');
        input.move_home();
        input.insert_char('a');
        assert_eq!(input.as_str(), "ab");
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_insert_char_at_middle() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('c');
        input.move_left();
        input.insert_char('b');
        assert_eq!(input.as_str(), "abc");
        assert_eq!(input.cursor_pos(), 2);
    }

    #[test]
    fn test_backspace_at_start_is_noop() {
        let mut input = InputState::new();
        input.backspace();
        assert_eq!(input.as_str(), "");
        assert_eq!(input.cursor_pos(), 0);

        input.insert_char('a');
        input.move_home();
        input.backspace();
        assert_eq!(input.as_str(), "a");
        assert_eq!(input.cursor_pos(), 0);
    }

    #[test]
    fn test_backspace_removes_previous_char() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        input.move_left();
        input.backspace();
        assert_eq!(input.as_str(), "ac");
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_backspace_at_end() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.backspace();
        assert_eq!(input.as_str(), "a");
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_delete_at_end_is_noop() {
        let mut input = InputState::new();
        input.delete();
        assert_eq!(input.as_str(), "");

        input.insert_char('a');
        input.delete();
        assert_eq!(input.as_str(), "a");
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_delete_removes_char_at_cursor() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        input.move_home();
        input.delete();
        assert_eq!(input.as_str(), "bc");
        assert_eq!(input.cursor_pos(), 0);
    }

    #[test]
    fn test_move_left_at_zero_is_noop() {
        let input = InputState::new();
        assert_eq!(input.cursor_pos(), 0);

        let mut input = InputState::new();
        input.insert_char('a');
        input.move_home();
        input.move_left();
        assert_eq!(input.cursor_pos(), 0);
    }

    #[test]
    fn test_move_left() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.move_left();
        assert_eq!(input.cursor_pos(), 1);
        input.move_left();
        assert_eq!(input.cursor_pos(), 0);
    }

    #[test]
    fn test_move_right_at_end_is_noop() {
        let mut input = InputState::new();
        input.move_right();
        assert_eq!(input.cursor_pos(), 0);

        input.insert_char('a');
        input.move_right();
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_move_right() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.move_home();
        input.move_right();
        assert_eq!(input.cursor_pos(), 1);
        input.move_right();
        assert_eq!(input.cursor_pos(), 2);
    }

    #[test]
    fn test_move_home() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        assert_eq!(input.cursor_pos(), 3);
        input.move_home();
        assert_eq!(input.cursor_pos(), 0);
    }

    #[test]
    fn test_move_end() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.move_home();
        assert_eq!(input.cursor_pos(), 0);
        input.move_end();
        assert_eq!(input.cursor_pos(), 2);
    }

    #[test]
    fn test_clear() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_pos(), 0);
        assert_eq!(input.as_str(), "");
    }

    #[test]
    fn test_set() {
        let mut input = InputState::new();
        input.set("hello".to_string());
        assert_eq!(input.as_str(), "hello");
        assert_eq!(input.cursor_pos(), 5);
    }

    #[test]
    fn test_is_empty() {
        let mut input = InputState::new();
        assert!(input.is_empty());
        input.insert_char('x');
        assert!(!input.is_empty());
        input.backspace();
        assert!(input.is_empty());
    }

    #[test]
    fn test_as_str() {
        let mut input = InputState::new();
        assert_eq!(input.as_str(), "");
        input.set("test".to_string());
        assert_eq!(input.as_str(), "test");
    }

    // Multi-byte character tests

    #[test]
    fn test_insert_multibyte_char() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('\u{00e9}'); // e-acute, 2 bytes
        input.insert_char('b');
        assert_eq!(input.as_str(), "a\u{00e9}b");
        assert_eq!(input.cursor_pos(), 4); // 1 + 2 + 1

        let mut input = InputState::new();
        input.insert_char('\u{4e16}'); // CJK character, 3 bytes
        assert_eq!(input.cursor_pos(), 3);

        let mut input = InputState::new();
        input.insert_char('\u{1f600}'); // emoji, 4 bytes
        assert_eq!(input.cursor_pos(), 4);
    }

    #[test]
    fn test_backspace_multibyte() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('\u{00e9}');
        input.insert_char('b');
        input.backspace();
        assert_eq!(input.as_str(), "a\u{00e9}");
        assert_eq!(input.cursor_pos(), 3);
        input.backspace();
        assert_eq!(input.as_str(), "a");
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_delete_multibyte() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('\u{00e9}');
        input.insert_char('b');
        input.move_home();
        input.move_right(); // past 'a'
        input.delete(); // delete e-acute
        assert_eq!(input.as_str(), "ab");
        assert_eq!(input.cursor_pos(), 1);
    }

    #[test]
    fn test_move_left_multibyte() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('\u{00e9}'); // 2 bytes
        input.insert_char('b');
        // cursor at 4 (end)
        input.move_left(); // back over 'b' (1 byte)
        assert_eq!(input.cursor_pos(), 3);
        input.move_left(); // back over e-acute (2 bytes)
        assert_eq!(input.cursor_pos(), 1);
        input.move_left(); // back over 'a' (1 byte)
        assert_eq!(input.cursor_pos(), 0);
    }

    #[test]
    fn test_move_right_multibyte() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('\u{00e9}'); // 2 bytes
        input.insert_char('b');
        input.move_home();
        input.move_right(); // past 'a'
        assert_eq!(input.cursor_pos(), 1);
        input.move_right(); // past e-acute (2 bytes)
        assert_eq!(input.cursor_pos(), 3);
        input.move_right(); // past 'b'
        assert_eq!(input.cursor_pos(), 4);
    }

    #[test]
    fn test_mixed_ascii_and_multibyte() {
        let mut input = InputState::new();
        // Build: "h\u{00e9}llo\u{1f600}"
        input.insert_char('h');
        input.insert_char('l');
        input.insert_char('l');
        input.insert_char('o');
        // Insert e-acute after 'h': move to position 1
        input.move_home();
        input.move_right();
        input.insert_char('\u{00e9}');
        assert_eq!(input.as_str(), "h\u{00e9}llo");
        // Append emoji
        input.move_end();
        input.insert_char('\u{1f600}');
        assert_eq!(input.as_str(), "h\u{00e9}llo\u{1f600}");
        // Navigate back and delete the e-acute
        input.move_home();
        input.move_right(); // past 'h'
        input.delete(); // remove e-acute
        assert_eq!(input.as_str(), "hllo\u{1f600}");
        // Backspace the emoji from the end
        input.move_end();
        input.backspace();
        assert_eq!(input.as_str(), "hllo");
    }
}
