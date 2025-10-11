use std::io;

pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .map_err(io::Error::other)
}
